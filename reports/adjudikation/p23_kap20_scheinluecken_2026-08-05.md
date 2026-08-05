# § 23 / § 20 — Scheinlücken-Prüfung 2026-08-05

**Datum:** 2026-08-05
**Status:** READ-ONLY, kein Code.
**HEAD:** 325f10d (Arbeitsbaum dreckig — dev-1 KV/PV-Migration läuft parallel)
**Auftrag:** Prüfung von 5 Feldern, die Julius als "keine Bindungsarbeit mehr" eingestuft hat.

---

## § 23 private Veräußerungsgeschäfte (3 Felder)

**Feld:** `p23_veraeusserungspreis` | Cent | askable | instanz_gruppe: p23_veraeusserung
**Feld:** `p23_anschaffung_herstellungskosten` | Cent | askable | instanz_gruppe: p23_veraeusserung
**Feld:** `p23_werbungskosten` | Cent | askable | instanz_gruppe: p23_veraeusserung
**Art-Weiche:** `p23_veraeusserungs_typ` (enum: grundstueck | anderes_wg)

### 1. Existiert ein Kz am XSD?

Ja. Das Schema hat ZWEI Kz-Ebenen:

**Per-Asset-Kz** (Einzelgeschäftsergebnis):
- `E0306801` — Pfad: `E10/SO/Priv_VA_G/Grdst/Einz/E0306801`
  xs:documentation: "Gewinn / Verlust"
- `E0307701` — Pfad: `E10/SO/Priv_VA_G/And_WG/Einz/E0307701`
  xs:documentation: "Gewinn / Verlust"

**Summen-Kz** (aggregiert über alle Assets, pro Person/Partnerschaft):
- `E0318902` — Pfad: `E10/SO/Priv_VA_G/Ant_Ek/Sum/E0318902`
  xs:documentation: "Summe" (für Anteile an Gemeinschaften/Gesellschaften)
  Die Walk-Dokumentation sagt nur "Summe". Der Container Ant_Ek trägt die Einzel-Kz E0318801/E0318802 für Gemeinschaft/Gesellschaft und Anteil-Gewinn/Verlust — das ist ein ANDERER Sachverhalt (Beteiligungen, nicht direkte §23-Veräußerungen).

Die Bindung notiert korrekt: "Kz E0318902 + 35 per-asset-Kz belegt" — das E0318902 ist das FALSCH identifizierte Summen-Kz. Es gehört zu Ant_Ek (Beteiligungen), nicht zu den direkten §23-Assets. Die echten per-asset Kz sind E0306801 (Grundstücke) und E0307701 (andere WG).

### 2. Passt es SEMANTISCH?

Die 3 Eingabe-Felder sind **Ring-Inputs** (Klasse c im Sinne von nicht_deklariert). Sie sind die Rohdaten, aus denen die Catala-Regel `p23_veraeusserungsgewinn` den Gewinn/Verlust je Asset errechnet. Das Kz gehört zum **Ergebnis** dieser Rechnung, nicht zu den Eingabe-Feldern.

- Brutto vs. netto: Der Veräußerungspreis ist ein Brutto-Wert (Eingabe), die Kz E0306801/E0307701 erwarten den Netto-Gewinn (errechnet). ✅ Semantisch passend als Ring-Output.
- Typ: Kz sind Cent (wie unser Rechenergebnis). Kein Ja/Nein-Feld, keine Prozent. ✅
- Aggregat vs. Einzelposten: Pro Asset → ein Eintrag in Grdst/Einz bzw. And_WG/Einz. Unsere instanz_gruppe p23_veraeusserung bildet genau das ab. ✅
- Gültigkeit: vz_gueltigkeit [2024, 2025, 2026] — Kz in allen drei Jahren vorhanden. ✅

### 3. Urteil: BINDBAR, aber NICHT auf den 3 Eingabe-Feldern

Die 3 Felder sind ENDGUELTIG (kein eigenes Kz nötig — Ring-Input). Das Kz muss auf den **berechneten per-Asset-Gewinn** gebunden werden:

```
p23_veraeusserungspreis ─┐
p23_anschaffungs_kosten ─┤  → p23_veraeusserungsgewinn (Catala)
p23_werbungskosten ──────┘  →
                            ↓
              ┌── Grundstück → E0306801
              │   (SO/Priv_VA_G/Grdst/Einz)
              └── Anderes WG → E0307701
                  (SO/Priv_VA_G/And_WG/Einz)
```

**Klasse:** Verzweigung f (wie rentner_jahresrente), art_feld = p23_veraeusserungs_typ.
**Besonderheit:** Der Kz-Wert ist NICHT der Roh-Feld-Wert, sondern das Rechenergebnis. Das braucht entweder (a) einen Fold in est_mapping (Klasse a: dokumentierte Aggregation? Nein — das ist keine Summe, sondern eine Differenz), oder (b) der Ring schreibt das Ergebnis in einen Store-Slot, den eine neue VERZWEIGUNG-Binding liest.

**Aufwands-Einschätzung:** ~1h (Verzweigung in est_mapping + ggf. einen intermediate-slot für das Rechenergebnis). Die Instanz-Achse (maxOccurs=2 pro Grdst/And_WG → Person A/B) ist bereits durch den person_b-Bucket gedeckt.

**Julius' "keine Bindungsarbeit" ist FALSCH.** Die 3 Eingabe-Felder selbst sind ENDGUELTIG, aber die Kz-Bindung des Rechenergebnisses ist offen.

---

## § 20 Kapital (2 Felder)

**Feld:** `kap_gewinn_sonstige` | Cent | askable
**Feld:** `kap_gewinn_sonstige_partner` | Cent | askable

### 1. Existiert ein Kz am XSD?

Wir haben am XSD folgende Kapital-Kz im Pfad `E10/KAP` (nur die Beträge lt. Steuerbescheinigung, nicht Korrekturen):

| Kz | Pfad (Betr_lt_StBesch) | xs:documentation | Gebunden? |
|----|----------------------|-----------------|-----------|
| E1900701 | KapErt_inl_StAbz/Betr_lt_StBesch/E1900701 | "Kapitalerträge" | ✅ (kap_kapitalertraege) |
| E1900901 | KapErt_inl_StAbz/Betr_lt_StBesch/E1900901 | "enthaltene Gewinne aus Aktienveräußerungen" | ✅ (kap_gewinn_aktien) |
| E1900904 | KapErt_inl_StAbz/Betr_lt_StBesch/E1900904 | "enthaltene Einkünfte aus Stillhalterprämien und Gewinne aus Termingeschäften" | ❌ (kein Feld) |
| E1900804 | KapErt_inl_StAbz/Betr_lt_StBesch/E1900804 | "enthaltene Gewinne aus Veräußerung bestandsgeschützter Alt-Anteile" | ❌ (kein Feld) |
| E1901101 | KapErt_inl_StAbz/Betr_lt_StBesch/E1901101 | "enthaltene Ersatzbemessungsgrundlage" | ❌ (kein Feld) |
| E1901201 | KapErt_inl_StAbz/Betr_lt_StBesch/E1901201 | "Nicht ausgeglichene Verluste ohne Verluste aus der Veräußerung von Aktien" | ✅ (kap_verlust_sonstige) |
| E1901301 | KapErt_inl_StAbz/Betr_lt_StBesch/E1901301 | "Nicht ausgeglichene Verluste aus der Veräußerung von Aktien" | ✅ (kap_verlust_aktien) |

**Kein Kz für "sonstige Kapitalgewinne" als separate Größe.**

Der relevante Kz-Baum ist:
```
E1900701 (Kapitalerträge, total)
├── E1900901 (darin enthalten: Aktiengewinne)
├── E1900904 (darin enthalten: Stillhalterprämien/Termingeschäfte)
├── E1900804 (darin enthalten: bestandsgeschützte Alt-Anteile)
└── E1901101 (darin enthalten: Ersatzbemessungsgrundlage)
(nicht enthalten: E1901201, E1901301 — das sind VERLUSTE, die der Abgeltungsteuer NICHT unterlagen)
```

### 2. Semantik-Prüfung

Die Bindungsbegründung sagt: "unser p20_6-Zuschnitt trennt 4 Töpfe; der Vordruck deklariert die SUMME der Kapitalerträge (E1900701) + die darin enthaltenen Aktiengewinne (E1900901). Der sonstige Gewinn ist deriviert (Summe − Aktien), kein eigenes Deklarationsfeld."

**Diese Begründung ist korrekt**, aber ich habe nachgemessen:

Unser internes Modell hat 4 Töpfe:
1. `kap_gewinn_aktien` → Aktien-Gewinne → E1900901 (Kz vorhanden ✅)
2. `kap_verlust_aktien` → Aktien-Verluste → E1901301 (Kz vorhanden ✅)
3. `kap_gewinn_sonstige` → Sonstige Gewinne → **KEIN Kz** ❌
4. `kap_verlust_sonstige` → Sonstige Verluste → E1901201 (Kz vorhanden ✅)

Plus `kap_kapitalertraege` = Total → E1900701 (Kz vorhanden ✅)

Das Schema erwartet:
- E1900701 = Total (unser `kap_kapitalertraege`)
- E1900901 = Aktiengewinne (unser `kap_gewinn_aktien`)
- E1901201 = Sonstige Verluste (unser `kap_verlust_sonstige`)
- E1901301 = Aktienverluste (unser `kap_verlust_aktien`)

Das Schema erwartet **kein** "sonstige Gewinne"-Kz. Die Information "was sind die sonstigen Gewinne" steckt in der Differenz E1900701 − E1900901 + E1901201 + E1901301 − (Stillhalter-/Alt-Anteil-/Ersatz-Kz). Nur: die drei fehlenden Kz (E1900904, E1900804, E1901101) haben wir auch nicht.

**Ist die Abbildung wirklich verlustbehaftet?** Aus unseren 4 Töpfen + total:
- E1900701 = kap_kapitalertraege → ✅
- E1900901 = kap_gewinn_aktien → ✅  
- E1901201 = kap_verlust_sonstige → ✅
- E1901301 = kap_verlust_aktien → ✅
- Die sonstigen Gewinne (ohne Aktien) sind in E1900701 enthalten, aber NICHT als separater Wert deklariert. Sie gehen in die Summe E1900701 ein → **verlustbehaftet nur für uns** (wir können den Wert nicht back-deklarieren), **aber die Deklaration ist korrekt**, weil das FA die Summe + Aktienaufteilung bekommt.

**Fachliche Bewertung:** Die Deklaration ist korrekt und vollständig — alle drei vom Schema verlangten Beträge (Gewinne, Aktiengewinne, Verluste) werden übermittelt. Die Tatsache, dass unser Modell einen Topf mehr hat, ist ein Modell-Problem, kein Deklarations-Problem. Der Wert `kap_gewinn_sonstige` ist ein interner Rechenwert (Input in die Verlustverrechnung), der nicht als eigenes Kz deklariert wird.

### 3. Urteil: MODELLPROBLEM — bestätigt

`kap_gewinn_sonstige` (und `_partner`) sind **ENDGUELTIG ohne Kz**. Der Wert geht in die Verlustverrechnung ein, die den `kap_kapitalertraege`-Wert (E1900701) produziert. Die Deklaration ist vollständig.

**Allerdings:** drei Kz im Schema haben wir KEIN Feld:
- E1900904 (Stillhalterprämien/Termingeschäfte)
- E1900804 (bestandsgeschützte Alt-Anteile)
- E1901101 (Ersatzbemessungsgrundlage)
Das sind Sub-Kategorie-Aufteilungen der Kapitalerträge, die bei der Prüfung durch das FA relevant sein können. Bisher deklarieren wir sie nicht (sie sind optional, minOccurs=0). Ein sauberer späterer Nachtrag wäre: "Sind in Ihren Kapitalerträgen Stillhalterprämien/Termingeschäfte enthalten?" → dann E1900904 befüllen. Das ist separate Arbeit.

---

## Zusammenfassung

| Feld | Bisher | Urteil | Kz | Begründung |
|------|--------|--------|----|-----------|
| p23_veraeusserungspreis | OFFEN (Kz-Arbeit) | ENDGUELTIG kein eigenes Kz | Ring-Input | Wert ist Rohdaten; das Kz gehört zum berechneten per-Asset-Gewinn |
| p23_anschaffung_herstellungskosten | OFFEN | ENDGUELTIG kein eigenes Kz | Ring-Input | wie oben |
| p23_werbungskosten | OFFEN | ENDGUELTIG kein eigenes Kz | Ring-Input | wie oben |
| → **berechneter per-Asset-Gewinn** (neues Kz-Objekt) | — | **BINDBAR** | E0306801 / E0307701 (Verzweigung f) | Die eigentliche Kz-Arbeit, die noch fehlt |
| kap_gewinn_sonstige | OFFEN (Modell-Mismatch) | ENDGUELTIG kein Kz | — | Wert geht in E1900701 (total) ein. Deklaration vollständig. |
| kap_gewinn_sonstige_partner | OFFEN | ENDGUELTIG kein Kz | — | Wie Person A |

**Fazit für Julius' These "keine Bindungsarbeit mehr":**
- §23: **Widerlegt.** 3 Eingabe-Felder sind ENDGUELTIG (Ring-Input), aber der berechnete per-Asset-Gewinn braucht noch Kz-Bindung via VERZWEIGUNG (E0306801/E0307701). Das ist ~1h Arbeit.
- §20: **Bestätigt.** `kap_gewinn_sonstige` und `_partner` sind echte Modellprobleme. Es gibt kein Kz für "sonstige Kapitalgewinne" — der Wert wird via E1900701 (total) + E1900901 (Aktiengewinne) + E1901201/1301 (Verluste) vollständig deklariert.

**Nebenfund:** Drei Kz im Schema (E1900904, E1900804, E1901101) haben wir kein Feld für — aber sie sind optional (minOccurs=0) und kein Blocker.