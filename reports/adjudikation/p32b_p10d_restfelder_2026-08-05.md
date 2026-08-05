# Restfelder p32b/p10d/p21/p33a — Adjudikation 2026-08-05

**Datum:** 2026-08-05
**Status:** READ-ONLY
**HEAD:** 6563f50 (Arbeitsbaum: dev-1 arbeitet in est_mapping.py)

---

## p32b_progressionseinkuenfte

**Urteil: BINDBAR — 1:1 auf E0104801**

| Kriterium | Befund |
|-----------|--------|
| Pfad | `E10/ESt1A/Eink_Ers/Inl/Sum/E0104801` |
| xs:documentation | "Einkommensersatzleistungen, die dem Progressionsvorbehalt unterliegen, z. B. Arbeitslosengeld, Elterngeld, Insolvenzgeld, Krankengeld, Mutterschaftsgeld, Verdienstausfallentschädigung (Infektionsschutz…)" |
| Kardinalität | `Sum` unter `Eink_Ers/Inl/…`: maxOccurs=1, minOccurs=0 |
| Passt semantisch? | ✅ Unser Aggregat (alle Progressionseinkünfte) → E0104801 ("Einkommensersatzleistungen … dem Progressionsvorbehalt unterliegen"). |
| Typ/Einheit | Cent → Cent (ceiling) ✅ |

**Prüfung auf alternative Zweige:** Die Sektion `ESt1A/Eink_Ers` hat zwei getrennte Unterpfade:

| Pfad | Kz | xs:documentation | maxOccurs |
|------|----|-----------------|-----------|
| Inl/Sum | **E0104801** | "Einkommensersatzleistungen, die dem Progressionsvorbehalt unterliegen" | 1 |
| Ausl/Sum | E0103910 | "Leistungen aus einem EU-/EWR-Staat oder der Schweiz, die vergleichbar sind" | 1 |

Unser Aggregat bildet BEIDE ab (Inland + Ausland werden addiert). E0104801 deckt nur Inland. E0103910 deckt Ausland. **Unser Aggregat ist verlustbehaftet** — wir können nicht zwischen Inland/Ausland trennen. Der Nutzer gibt eine Summe, die über zwei Kz verteilt werden müsste.

Allerdings: praktisch haben fast alle Nutzer nur Inlandsleistungen (deutsche SV-Träger melden elektronisch). Auslandsleistungen erfordern eine gesonderte Bescheinigung. Die Bindung notiert selbst: "Auslandsleistungen meldet ohnehin kein deutscher SV-Träger."

**Empfehlung:** E0104801 binden (Default-Inland). Einen zweiten Zweig für E0103910 nur, wenn ein Nutzer-Feld dafür existiert. Separates Ticket (~0.5h).

**Aufwand:** ~15 Minuten (E0104801 in est_mapping als 1:1 für p32b_progressionseinkuenfte).

---

## verlustvortrag_bestand

**Urteil: KEIN Kz — Typ-Mismatch (Ja-Feld)**

| Kriterium | Befund |
|-----------|--------|
| Pfad | `E10/Sonst/Verl_Abz/Vortrag/E0190701` |
| xs:documentation | "Es wurde ein verbleibender Verlustvortrag nach § 10d EStG zum 31.12.$VZ-1$ festgestellt." |
| XSD-Typ | `Ja1BaseCType_RABE` — Ja/Nein-Feld, kein cent |
| Passt semantisch? | ❌ Unser Feld ist cent-Betrag (Höhe des Verlustvortrags). E0190701 ist Ja/Nein (ob ein Vortrag FESTGESTELLT wurde). |
| Alternativen geprüft? | Kein Betrag-Kz unter `E10/Sonst/Verl_Abz/` gefunden. Die anderen Kz dort sind: E0109403 "Betrag" (Rücktrag § 2b), E0190802 "absehen beantragen" (Ja/Nein), E0301514 (SO/Leist/Verl_Abz). Keines bildet "Höhe des verbleibenden Verlustvortrags" ab. |

**Bindungsgrund bestätigt:** Unser cent-Feld kann nicht auf das Ja/Nein-Kz E0190701 gemappt werden. Der Betrag ist dem FA aus dem Feststellungsbescheid bekannt (§ 10d Abs. 4) — das E10 fragt ihn nicht ab, weil ihn das FA selbst hat.

**Frage aus der Bindung:** ob wir E0190701 als **abgeleitetes Ja/Nein** (bestand > 0) setzen sollten. Das ist schema-konform (Ja/Nein ist der korrekte Typ) und wäre eine Deklarations-Verbesserung: das FA bekommt das Signal "ein Vortrag wurde festgestellt" auch dann, wenn der Betrag 0 sein sollte (was denkbar ist). **Nicht dringend** — das FA hat den Feststellungsbescheid. Aber einfach und billig.

**Aufwand:** ~10 Minuten (Negation-Kz in est_mapping: "verlustvortrag_bestand > 0 → E0190701 = True"). Over-tax-safe: ohne Setzen fehlt das Ja, das FA hat aber die Feststellung selbst.

---

## vv_entgelt_quote_prozent

**Urteil: MODELLPROBLEM — nicht direkt bindbar**

| Kriterium | Befund |
|-----------|--------|
| Container | `E10/V/Verbilligt` (maxOccurs=1, minOccurs=0) |
| Kz 1: **E0708601** | `E10/V/Verbilligt/E0708601` — xs:documentation: "Kürzung der Werbungskosten wegen verbilligter Vermietung (in %)" |
| Kz 2: **E0708701** | `E10/V/Verbilligt/E0708701` — xs:documentation: "Betragsmäßige Kürzung der Werbungskosten wegen verbilligter Vermietung eines Teils des Objekts" |

Beide minOccurs=0, maxOccurs=1.

**Bindungsgrund bestätigt:** Unser Feld ist die ENTGELT-Quote (Miete/Marktmiete × 100, 0-100%). E0708601 ist die KÜRZUNGS-Quote — die Gegengröße. Bei 60% Entgelt-Quote beträgt die Kürzungs-Quote NICHT 40% linear, sondern folgt der 50/66%-Staffel des § 21 Abs. 2:

| Entgelt-Quote | WK-Abzug |
|---------------|---------|
| ≥ 66% | voll (keine Kürzung) |
| > 50% und < 66% | anteilig |
| ≤ 50% | nur anteilig (aufgeteilt) |

Die Berechnung ist nicht linear: E0708601 = Kürzungsprozentsatz, der sich aus der Staffel ergibt. Unser Wert ist der Eingabe-Prozentsatz. Der Kürzungsprozentsatz müsste über die Catala-Regel `p21_2_verbilligte_vermietung_wk` berechnet werden (die kennt die Staffel). Das ist kein Mapping-Problem, sondern ein Rechenoutput-Problem.

**Empfehlung:** `vv_entgelt_quote_prozent` bleibt ENDGUELTIG ohne Kz (Ring-Input). Falls der Rechenoutput (Kürzungsbetrag) je Objekt ein Kz braucht, müsste der Ring den an die Deklaration reichen — das ist genauso aufwendig wie `einkuenfte_gewinn` (Verzweigung f). ~1h wenn gewünscht.

**Aufwand:** 0 für Status quo. ~1h für Rechenoutput-Kz.

---

## p33a_ausbildung_anzahl_kinder

**Urteil: MODELLPROBLEM — Anzahl ≠ Kz**

Das Feld zählt Kinder (int, 0-20). Die Kz unter `E10/Sonst/Unterhalt/` betreffen BETRÄGE, nicht Anzahlen. Kein Kz für "Anzahl der Kinder in Ausbildung" im Walk gefunden.

**Die Anlage Unterhalt (Sonst/Unterhalt) hat:**
- Einz/Sum-Struktur (wie Ges_Fest) — Beträge, Bezeichnung, Finanzamt, Steuernummer pro unterstützter Person
- Kein Kz für "Anzahl Kinder"

**Warum das KEIN Problem ist:** Der Ausbildungsfreibetrag (§ 33a Abs. 2, 1.200 € je Kind) wird vom Ring berechnet: `anzahl_kinder × 1.200 €`. Das Ergebnis — der Freibetrag — geht in die Steuerberechnung ein, NICHT in eine separate Kz-Deklaration. Der Betrag IST kein Sonderausgaben-Kz, sondern ein Abzug bei den Einkünften (§ 33a Abs. 1: "bei der Ermittlung der Einkünfte abzuziehen").

**Struktur der Anlage Unterhalt:**
- `Sonst/Unterhalt/Pers` (person-individuell, maxOccurs=2): E0124601 Art, E0124701 Betrag, …
- `Sonst/Unterhalt/Einz` (pro unterstützte Person, maxOccurs=99): E0183101 Name, E0181802 Betrag, …

Unser Feld `p33a_ausbildung_anzahl_kinder` ist Ring-Input für die Berechnung. Die Kz gehen an den BETRAG (p33a_unterhalt_aufwendungen → E0120103, bereits gebunden in Block 3), nicht an die Anzahl.

**Aufwand:** 0. Status quo ist korrekt.

---

## Zusammenfassung

| Feld | Urteil | Kz | Begründung |
|------|--------|----|-----------|
| p32b_progressionseinkuenfte | **BINDBAR** | E0104801 | 1:1 auf Inlands-Leistungen. Auslands-Kz E0103910 optional. ~15min. |
| verlustvortrag_bestand | KEIN Kz | — | E0190701 ist Ja/Nein, kein Cent. Betrag bekannt vom FA. Optional: abgeleitetes Ja/Nein (10min). |
| vv_entgelt_quote_prozent | MODELLPROBLEM | — | Kz (E0708601) ist Kürzungs-%, nicht Entgelt-%. Ring berechnet Staffel; Rechenoutput bräuchte separaten Kz (~1h). |
| p33a_ausbildung_anzahl_kinder | RING-INPUT | — | Anzahl, kein Kz. Der Betrag (p33a_unterhalt_aufwendungen) ist bereits gebunden. |

**Bindbar:** **1 von 4** (p32b). Verlustvortrag und Quote sind echte Modellprobleme. p33a_anzahl ist korrekt ohne Kz.

**Anmerkung:** die Bindung nennt E0104801 bereits im elster_kz_grund — damit ist der Kandidat vorbereitet. Einfach in est_mapping eintragen.