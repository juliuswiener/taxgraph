# Degenerate-Anker — Migrations-Vorlage für dev-1 (taxgraph-dev-2, 2026-07-15)

Instructor-Auftrag: je Hartfall (1× p35_1 + 7× Cross-Source) die konkrete Migrations-Zeile
ausarbeiten (Volllängen-Fragment + datei-Zeiger, gegen Freeze getestet), p35_1 doppelt vorbereiten,
17 Kurz-Anker Diskriminanz prüfen, Zähl-Abgleich 263↔260. Read-only, `_normalize` exakt aus
`gates.py:828`. Skripte: `scratchpad/migration_vorlage.py` + `anker_kontext.py` + `anker_final.py` ($0).
KEIN Eingriff in rules.yaml/pipeline — reine Vorlage; dev-1 übernimmt beim D0+D1+D4-Bau.

## 4. ZÄHL-ABGLEICH (263 grep vs 260 Einträge) — GEKLÄRT

`grep -c "deckt_ab" rules.yaml = 263` zählt alle Zeilen mit dem Token. Zerlegung (verifiziert):

| Quelle der Zeile | Zahl | |
|---|---|---|
| `^  deckt_ab:` — Key in **geltungsbedingungen** | 260 | die eigentliche Bestandsmenge |
| `^  - deckt_ab:` — erster Key eines **rundung**-Listen-Items | 3 | **anderes Feld** (Zeilen 1154/1249/1564) |
| **Summe grep** | **263** | |

Die 3 Extra-Zeilen sind `rundung.deckt_ab` (bei solzg §4 SolzG, p36 §36 III, p10 §10 III) — NICHT
geltungsbedingungen. Sie tragen je einen eigenen `zitatanker`, der bereits freeze-geprüft wird
(`gates.py:300`) → kein Migrations-Gegenstand.

Innerhalb der **260 geltungsbedingungen-deckt_ab**: 257 String-Wert + 3 Listen-Wert
(`p9_4a`×2 + `p35a_2_3`×1, je 2 Fragmente). Fragmente aufgelöst = 257 + 6 = **263**.

→ **Für den Gate-Commit gilt: 260 Bedingungs-Anker / 263 Fragmente** (die 263 im Erst-Report war die
Fragment-Zahl; „260 Einträge" die Bedingungs-Zahl — beide korrekt, nur unterschiedliche Einheit).
Dass grep-Zeilen (263) und Fragmente (263) beide 263 sind, ist Zufall verschiedener Zerlegung.

## 1./2. HARTFÄLLE — Migrations-Zeile je Fall (alle _normalize-getestet gegen Freeze)

**Cross-Source (7): `datei`-Zeiger ergänzen, Fragment bleibt unverändert (alle OK, Volllänge).**
Migration = je Bedingung ein `datei:`-Feld (D1-Default-Override) auf die Norm, die das Zitat trägt:

| rule_id | bedingung | Fragment (Ist, verbatim OK) | Länge | **datei-Zeiger (neu)** |
|---|---|---|---|---|
| p4_3_gewinn | keine_lebensfuehrungskosten_p12_nr1 | „die für den Haushalt des Steuerpflichtigen und für den Unterhalt seiner Familienangehörigen aufgewendeten Beträge" | 117 | `sources/gesetze-im-internet/estg_p12_2026-07-14.txt` |
| p4_3_gewinn | keine_repraesentation_gemischt_p12_nr1s2 | „Aufwendungen für die Lebensführung, die die wirtschaftliche oder gesellschaftliche Stellung des Steuerpflichtigen mit sich bringt" | 131 | `…/estg_p12_2026-07-14.txt` |
| p4_3_gewinn | keine_personensteuern_p12_nr3 | „die Steuern vom Einkommen und sonstige Personensteuern" | 54 | `…/estg_p12_2026-07-14.txt` |
| p4_3_gewinn | einkunftsart_gewerbe_p15_abs2 | „selbständige nachhaltige Betätigung, die mit der Absicht, Gewinn zu erzielen, unternommen wird und sich als Beteiligung am allgemeinen wirtschaftlichen Verkehr darstellt" | 171 | `…/estg_p15_2026-07-14.txt` |
| p4_3_gewinn | einkunftsart_selbstaendige_arbeit_p18_abs1 | „Einkünfte aus selbständiger Arbeit sind" | 41 | `…/estg_p18_2026-07-14.txt` |
| p34c_1_anrechnung_hoechstbetrag | auslaendische_einkuenfte_p34d_katalog | „Ausländische Einkünfte im Sinne des § 34c Absatz 1 bis 5 sind" | 63 | `…/estg_p34d_2026-07-14.txt` |
| p4_1_bv_vergleich | massgeblichkeit_handelsbilanz_gob | „das nach den handelsrechtlichen Grundsätzen ordnungsmäßiger Buchführung auszuweisen ist" | 91 | `…/estg_p5_2026-07-14.txt` |

Alle 7 Fragmente: `_normalize`-Test **OK** in der jeweils genannten Datei, Volllänge 41–171 Zeichen,
diskriminierend. Migration rein additiv (nur `datei:`-Zeile je Bedingung).

## 2. p35_1 — doppelt vorbereitet

**(a) Faltet `_normalize` Bindestriche? NEIN.** `_normalize` (gates.py:828) macht nur Umlaut-
Transliteration + lowercase + `\s+`→Space + strip. „-" ist kein `\s` → bleibt erhalten. Test:
`_normalize("Gewerbesteuermessbetrags") = "gewerbesteuermessbetrags"` (ohne Bindestrich) vs
Quelle `_normalize("Gewerbesteuer-Messbetrags") = "gewerbesteuer-messbetrags"` (mit Bindestrich) →
**kein Match**. Ist-Anker `"Gewerbesteuermessbetrags"` FEHLT im Freeze (0 Treffer). Der §35-Quelltext
schreibt durchgängig „Gewerbesteuer-Messbetrag(s)" mit Bindestrich + großem M (7×) plus 1×
„Steuermessbetrags". Der Ist-Anker ist ein zusammengeschriebenes Kompositum, das so nicht existiert.

**(b) Korrigierter Verbatim-Anker (getestet):**
`deckt_ab: "Steuermessbetrags (Gewerbesteuer-Messbetrag)"` — **1× eindeutig**, 44 Zeichen, verbatim
in `sources/gesetze-im-internet/estg_p35_2026-07-14.txt` (die definitorische Klammer in § 35 Abs. 1
Nr. 1). Ersetzt den degenerierten 24-Zeichen-Ist-Anker durch einen diskriminierenden Volllängen-Anker.
(Verworfen: bloßes „Gewerbesteuer-Messbetrags" = 7× mehrdeutig; „…anteiligen Gewerbesteuer-Messbetrags"
= FEHLT in genau der Schreibweise.)

## 3. KURZ-ANKER (<25 Zeichen) — Diskriminanz + Ersatz (D4-Lint-Baseline)

17 Fragmente <25 Zeichen normalisiert; Match-Zahl in der REGEL-EIGENEN Quelle:

| Diskriminanz | Fall | Anker | Befund / Empfehlung |
|---|---|---|---|
| **0× FEHLT** | p35_1 messbetrag_gewst_sachverhalt | „Gewerbesteuermessbetrags" | → Hartfall, s. 2(b) |
| **3× MEHRDEUTIG** | p35a_2_3 antrag_gestellt | „auf Antrag" | → **D1-Multi-Fragment** (s. u.) |
| 1× eindeutig | solzg splitting_ist_veranlagungsergebnis | „40 700 Euro" | kurz aber eindeutig; T3-Tabellenkonstante → in D1 umgebenden Satz ankern oder D4-Waiver |
| 1× eindeutig | p10_1_4 keine_zuschlagsteuer_kappung | „gezahlte Kirchensteuer" | eindeutig; D4-Waiver oder verlängern |
| 1× eindeutig | p10_1_4 kein_erstattungsueberhang | „gezahlte Kirchensteuer" | dito (2× dieselbe Regel — Namen unterscheiden, Anker gleich) |
| 1× eindeutig | p32d_1_abgeltung keine_kirchensteuer_auf_kapitalertraege | „beträgt 25 Prozent" | eindeutig; D4-Waiver |
| 1× eindeutig | p32d_1_kirchensteuer keine_guenstigerpruefung_beantragt | „beträgt 25 Prozent" | eindeutig; D4-Waiver |
| 1× eindeutig | p34_3 antrag_gestellt | „so kann auf Antrag" | eindeutig |
| 1× eindeutig | p34_3 einmal_im_leben | „nur einmal im Leben" | eindeutig |
| 1× eindeutig | p6_2a wahlrecht_einheitlich_pro_wj | „einheitlich anzuwenden" | eindeutig |
| 1× eindeutig | p16_4 freibetrag_einmal_im_leben | „nur einmal zu gewähren" | eindeutig |
| 1× eindeutig | p16_4 auf_antrag | „auf Antrag" | eindeutig (in dieser Quelle) |
| 1× eindeutig | p34c_2 antrag_abzug_statt_anrechnung | „Statt der Anrechnung" | eindeutig |
| 1× eindeutig | p6_1_3a ausnahme_verzinslich | „die verzinslich sind" | eindeutig |
| 1× eindeutig | p15a_1 erweitert_p2_subsumtion | „abweichend von Satz 1" | eindeutig |
| 1× eindeutig | estr_4_6 uebergangsgewinn_saldo_input | „der Übergangsgewinn" | eindeutig |
| 1× eindeutig | estr_4_6 uebergangsgewinn_positiv | „der Übergangsgewinn" | eindeutig (2× dieselbe Regel) |

**Nur 2 der 17 sind wirklich problematisch** (p35_1 fehlt, p35a mehrdeutig); die übrigen 15 sind
kurz-aber-eindeutig → für D4 genügt ein `schwach_ok: true`-Waiver mit Begründung ODER optional
Verlängerung. Der D4-Lint sollte also Länge UND Diskriminanz (Match-Zahl) prüfen — reine Länge würde
15 harmlose Fälle unnötig flaggen.

**p35a antrag_gestellt — D1-Multi-Fragment-Vorlage (getestet, alle 1× eindeutig):**
Die Bedingung deckt den Antrag über alle drei Absätze (§ 35a Abs. 1/2/3), daher legitim mehrfach:
```
deckt_ab:
- "auf Antrag um 20 Prozent, höchstens 510 Euro"                 # Abs. 1  (45Z, 1×)
- "auf Antrag um 20 Prozent, höchstens 4 000 Euro"               # Abs. 2  (47Z, 1×)
- "auf Antrag um 20 Prozent der Aufwendungen des Steuerpflichtigen" # Abs. 3 (63Z, 1×)
datei: sources/gesetze-im-internet/estg_p35a_2026-07-09.txt
```
Ersetzt das mehrdeutige 10-Zeichen-„auf Antrag" durch drei diskriminierende Absatz-spezifische
Volllängen-Fragmente — Musterfall für D1-Multi-Fragment.

## Zusammenfassung für dev-1s D0+D1+D4-Bau
- **7 Cross-Source:** je `datei:`-Zeile ergänzen (Tabelle oben), Fragmente unverändert. Rein additiv.
- **p35_1:** `deckt_ab` auf `"Steuermessbetrags (Gewerbesteuer-Messbetrag)"` korrigieren (Ist-Anker fehlt
  im Freeze wegen Bindestrich-Schreibweise).
- **p35a:** auf D1-Multi-Fragment (3 Absätze) umstellen.
- **15 weitere Kurz-Anker:** eindeutig → D4-Waiver genügt; D4-Lint auf Länge **UND** Match-Zahl prüfen.
- **Bestandszahl Gate-Commit:** 260 Bedingungs-Anker / 263 Fragmente (rundung.deckt_ab ×3 separat,
  bereits zitatanker-geprüft).
- Alle Fragment-Anker in dieser Vorlage sind `_normalize`-getestet gegen den Freeze (OK/FEHLT je Zeile).

Kein Eingriff in rules.yaml/pipeline (dev-1-TABU). gates.py/quellen.py-Bau = dev-1-Zone.
