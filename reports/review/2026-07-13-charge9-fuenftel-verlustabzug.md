# Charge 9 — Zuschnitt § 34 Fünftelregelung + § 10d Verlustabzug (Stufe A)

Datum: 2026-07-13 · Formalisierer-Ziel: 2 Regeln · Freeze sha256 verifiziert
(p34 `4d3337ee…` ✓, p10d `7b86a8a7…` ✓, sources-check 58/58).

## ⚠ KORREKTUR zur Instructor-Rahmung: § 10d = 70 %, NICHT 60 %

Rahmung msg 1648 sagt „1 Mio Sockel + **60 %** des übersteigenden GdE". Der **eingefrorene
Wortlaut Abs 2 S 1** sagt **70 Prozent**:

> „bis zu einem Gesamtbetrag der Einkünfte von 1 Million Euro unbeschränkt, darüber hinaus bis
> zu **70 Prozent** des 1 Million Euro übersteigenden Gesamtbetrags der Einkünfte"

60 % war die alte Mindestbesteuerungsgrenze; das Wachstumschancengesetz hob sie für VZ 2024–2027
auf 70 % an (geltende Fassung 2026). **Wortlaut hat Vorrang** → Regel rechnet mit 70/100.
Grenz-Nachtrag: ab VZ 2028 fällt sie planmäßig auf 60 % zurück — VZ-Versionierung, nicht Charge 9.

---

## Regel 1 — `p34_fuenftel_ao_est` (§ 34 Abs 1: Fünftelregelung)

**Zuschnitt = reine Tarif-Differenz-Andockung** (§ 31/§ 32b-Muster, Instructor bestätigt). Beide
Tarifwerte kommen als Inputs aus der § 2-Integration; die Regel rechnet NUR die Differenz-×5-Formel,
KEIN Selbst-Tarif.

| Feld | Wert |
|---|---|
| Inputs | `est_verbleibendes_zve` money, `est_verbleibendes_zve_plus_fuenftel_ao` money |
| Output | `est_ao` money (auf die ao-Einkünfte entfallende ESt) |
| Formel | `est_ao = 5 × (est_verbleibendes_zve_plus_fuenftel_ao − est_verbleibendes_zve)` |
| Zitatanker | Abs 1 S 2 „beträgt das Fünffache des Unterschiedsbetrags zwischen der Einkommensteuer für das … verbleibende zu versteuernde Einkommen … und der Einkommensteuer für das verbleibende zu versteuernde Einkommen zuzüglich eines Fünftels dieser Einkünfte" |

**hinweis (Andockung-Pin, Klasse 1 — Kontext-Hunger-Sperre wie § 32b):**
> `est_ao = 5 × (est_verbleibendes_zve_plus_fuenftel_ao − est_verbleibendes_zve)`. BEIDE ESt-Werte
> sind INPUTS (Tarif-Andockung § 31/§ 32b-Muster) — KEIN Selbst-Tarif, KEINE Tariffunktion in dieser
> Regel. Faktor 5 exakt (Struktur-Konstante, NICHT /100). Kein max(0)-Boden (Differenz konstruktiv ≥ 0
> bei verbleibendem zvE ≥ 0).

**Seeds:** `(8000, 9200)→6000` · `(8000, 8000)→0` (ao=0/keine ao-Einkünfte) · `(12000, 13500)→7500`.

**Geltungsbedingungen (deklariert, nicht formalisiert):**
- `ao_einkuenfte_zusammengeballt` — Abs 2 Katalog: Veräußerungsgewinne (Nr 1), **Entschädigungen § 24 Nr 1 = Abfindungen (Nr 2, häufigster AN-Fall)**, Nachzahlungen >3 J (Nr 3), mehrjährige Vergütungen (Nr 4). Die ao-Qualifikation/Zusammenballung = Input-Semantik, kein Regel-Gegenstand.
- `verbleibendes_zve_nicht_negativ` — **Abs 1 S 3 Sonderpfad ausgeklammert** (wenn vzvE<0 ∧ zvE>0: `est_ao = 5 × est(zvE/5)`). Verlust-Sonderfall, AN-fern, braucht dritten Tarif-Input → Backlog-Grenze.

**Ausgeklammert (dokumentierte Grenzen):** Abs 3 (ermäßigter 56-%-Durchschnittssatz ab 55. LJ / Berufsunfähigkeit, einmal im Leben, nur Betriebsveräußerung) — antragsabhängiger Sonder-Tarifpfad, AN-fern, eigener Nachtrag falls je gewünscht.

---

## Regel 2 — `p10d_2_verlustvortrag_abzug` (§ 10d Abs 2: Verlustvortrag-Höchstbetrag)

**Zuschnitt = Höchstbetrags-Mechanik pro VZ** (Instructor-Rahmung korrekt, nur %-Zahl korrigiert).
Der MEHRJAHRES-STATE (Bestands-Fortschreibung Abs 4, gesonderte Feststellung) = § 2-Integrations-/
State-Territorium (Rentenfreibetrag-Präzedenz), NICHT Regel-Gegenstand.

| Feld | Wert |
|---|---|
| Inputs | `gesamtbetrag_einkuenfte` money, `verlustvortrag_bestand` money, `ist_zusammenveranlagt` bool |
| Output | `verlustabzug` money |
| Formel | `sockel = ist_zusammenveranlagt ? 2 000 000 : 1 000 000` (Norm-Konstante); `verlustabzug = min(verlustvortrag_bestand, sockel + (70/100) × max(0, gesamtbetrag_einkuenfte − sockel))` |
| Zitatanker | Abs 2 S 1 „bis zu einem Gesamtbetrag der Einkünfte von 1 Million Euro unbeschränkt, darüber hinaus bis zu 70 Prozent des 1 Million Euro übersteigenden Gesamtbetrags der Einkünfte" + S 2 (2 Mio bei Zusammenveranlagung) |

**Konstanten-Doktrin-Check:** Sockel 1/2 Mio + 70 % = **Norm-Konstanten** (kein Signature-Input —
caller darf Norm nicht verstellen). `ist_zusammenveranlagt` = Sachverhalt → Input-bool (Sparer-PB-
Präzedenz p20_9: Fallunterscheidung intern). `verlustvortrag_bestand` = computed State (Vorjahr) →
Input. `gesamtbetrag_einkuenfte` = Input aus § 2. ✓ doktrinkonform.

**hinweis (/100-Encoding Klasse 1 + Sockel-Konditional Klasse 2):**
> `sockel = wenn ist_zusammenveranlagt dann 2000000€ sonst 1000000€` (Norm-Konstante nach Veranlagung).
> `verlustabzug = min(verlustvortrag_bestand, sockel + (70/100) × max(0, gesamtbetrag_einkuenfte − sockel))`.
> 70 ist PROZENT → /100 (%-Tabellen-Leitlinie). Bestand ist Input/State; Mehrjahres-Fortschreibung
> (Abs 4) gehört NICHT in diese Regel. Bestand ≥ 0 vorausgesetzt.

**Seeds:**
- `(GdE 500000, bestand 800000, single)` → sockel 1Mio, GdE<sockel → cap 1Mio; `min(800000, 1000000)=800000` (voller Bestand)
- `(GdE 3000000, bestand 5000000, single)` → cap `1Mio + 0.7×2Mio = 2.4Mio`; `min(5Mio, 2.4Mio)=2400000` (70%-Staffel greift)
- `(GdE 3000000, bestand 5000000, zusammen)` → sockel 2Mio, cap `2Mio + 0.7×1Mio = 2.7Mio`; `=2700000` (Veranlagungs-Verzweigung)
- `(GdE 3000000, bestand 1500000, single)` → cap 2.4Mio; `min(1.5Mio, 2.4Mio)=1500000` (Bestand kappt)
- `(GdE 3000000, bestand 0, single)` → `0` (kein Vortrag)

---

## Verlustrücktrag (§ 10d Abs 1) — VORSCHLAG: dokumentierte Grenze, KEINE Regel in Charge 9

Begründung:
1. **Fremd-VZ-Eingriff:** Abs 1 S 4–5 ändert den Steuerbescheid des VORangegangenen VZ → reines
   Verfahrens-/Mehrjahres-State-Territorium (§ 2-Integration + AO-Bescheidänderung), noch stärker
   State-gebunden als der Vortrag (der nur den aktuellen GdE mindert).
2. **AN-fern:** Verlustrücktrag ist Selbständigen-/Vermietungs-Terrain; AN-Verluste selten, und dann
   greift meist der Vortrag. Der häufige AN-nahe Kern = Vortrag (Abs 2) → formalisiert.
3. **Sinnleer ohne State:** Isolierte Kappung `min(neg_eink, sockel)` ist trivial formalisierbar,
   aber ohne Ziel-VZ (wohin der Betrag fließt) semantisch leer.

**Falls Instructor Mini-Regel wünscht:** `p10d_1_ruecktrag_hoechstbetrag = min(nicht_ausgeglichene_negative_einkuenfte, sockel)` — reine 1/2-Mio-Kappung OHNE 70 %-Staffel (Abs 1 hat keine Prozent-Staffelung, nur absolute Deckelung). Antragsverzicht Abs 1 S 6 = Geltungsbedingung. Empfehlung bleibt: dokumentierte Grenze.

---

## Zusammenfassung Stufe A

| Regel | Kern | Inputs | Besonderheit |
|---|---|---|---|
| `p34_fuenftel_ao_est` | 5×Tarif-Differenz | 2 ESt-Werte (Andockung) | S3-Verlustpfad + Abs3-56%-Satz ausgeklammert |
| `p10d_2_verlustvortrag_abzug` | min(Bestand, Sockel+70%×Überhang) | GdE, Bestand, Veranlagung | **70 % (nicht 60)**; Rücktrag+Fortschreibung ausgeklammert |

Bereit für Stufe B (~$0,1). Rücktrag-Entscheidung (dokumentierte Grenze vs. Mini-Regel) erbeten.
