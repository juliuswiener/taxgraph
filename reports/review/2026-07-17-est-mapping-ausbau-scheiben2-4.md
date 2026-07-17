# est_mapping-Ausbau Scheiben 2-4 (Task #11, Paket A)

**Ziel:** die Deklarations-Seite zieht mit der Fragebogen-Breite nach — die 1:1-Kz der späteren
Scheiben werden gemappt + Round-Trip-gelockt; §21-WK bleibt bewusste Nicht-Deklaration.
**Keine** Bindungstabellen-/Engine-Änderung. fail-closed / Guard D / Store-CENT unverändert.

## Was gemappt ist (1:1 über `bindung.elster_kz`, generisch)

| Scheibe | Kz | Feld | Klasse |
|---|---|---|---|
| Kapital §20 | E0121709 | kap_kapitalertraege | 1:1 |
| Kapital §20 | E1900901 | kap_gewinn_aktien | 1:1 (Teilmenge von E0121709) |
| Kapital §20 | E1901301 | kap_verlust_aktien | 1:1 |
| Kapital §20 | E1901201 | kap_verlust_sonstige | 1:1 |
| V+V §21 | E0700201 | vv_einnahmen (Mieteinnahmen) | 1:1 |
| §35a/agB | E0104109 | agb_aufwendungen | 1:1 |
| §35a | E0161404 / E0161504 / E0161804 | hh_minijob / hh_dienstleistungen / hh_handwerker_arbeitskosten | 1:1 |

Die 5 Fall-Klassen sind unverändert; die 1:1-Klasse liest `bindung.elster_kz` — die neuen Kz sind
damit ohne Sonderfall abgedeckt, die Tests LOCKEN sie (Regressionswächter).

## Semantik-Entscheide

- **Aktien-Subset (E1900901 ⊂ E0121709):** beide werden EINZELN deklariert (Vordruck-Memo für die
  Verlustverrechnung); est_mapping mappt jedes 1:1, die Subset-Beziehung (`gewinn_aktien ≤
  kapitalerträge`) ist Validierungs-, nicht Transform-Sache. Test hält sie testdaten-konsistent.
- **§21-WK-Aggregation = DOKUMENTIERT, NICHT deklariert** (dein Anlage-V-Ruling): die 4 Detail-Slots
  (afa/schuldzinsen/erhaltung/sonstige) summieren auf E0703838, das die E10-Submission NICHT als
  sauberes Einzel-Kz führt (Zuordnungsart Direkt/Verhaelt, Mehrzeilen je Objekt). Umbau: die Summe
  wandert von `deklaration` in einen neuen **`dokumentiert`**-Bucket (`{E0703838: {summe, quell_felder}}`);
  der Round-Trip prüft die Summe aus `dokumentiert` (aggregat-genau), nie die Details. Konsistent mit
  der Bindungstabelle (dort `elster_kz=null`+Grund für dieselben Felder).
- **Rentner/fam null-Kz = Deklarations-Lücke:** alle Rentner-Felder + fam-Negation/Zähl gehen
  maschinenlesbar mit Grund nach `nicht_deklariert` (Anlage-R/Kind-Kz = dein Freeze-Nachtrag, offen);
  keine erfundene E-Nr.

## Gate (tests/test_est_mapping.py: 13 → 18)

Neu/geändert: `test_klasse_a_dokumentiert_nicht_deklariert` (E0703838 NICHT in deklaration, Σ in
`dokumentiert`), Round-Trip + Negativ auf `dokumentiert` umgestellt; **neu**
`test_scheibe3_kapital_und_vv_1zu1_roundtrip`, `test_aktien_subset_semantik_beide_deklariert`,
`test_scheibe2_sonder_35a_agb_1zu1_roundtrip`, `test_scheibe4_rentner_null_kz_gap`,
`test_neg_scheibe3_verfaelschtes_1zu1_bricht_roundtrip`. Fresh-Store-Helfer `_store_mit` (isoliert von
`_voller_store`, wahrt One-Active-Event/Feld).

**Volle Paket-A-Suite mit Catala-Toolchain: 80/80 grün, 0 skips.** fail-closed/Guard-D/Determinismus/
Konsistenz unverändert grün.

## Response-Key-Änderung (dev-1-Koordinationspunkt, Zonen)

`est_mapping.deklariere` gibt jetzt **`dokumentiert`** statt `lossy` zurück (Aggregation ist nicht mehr
deklariert). dev-1s `deklaration`-Endpoint (`produkt/haut/api.py:265`) spreadet `**result` verbatim →
**kein Code-Bruch**; nur die Response-JSON trägt `dokumentiert` statt `lossy`. `produkt/haut/api_schema/`
führt `lossy` NICHT (geprüft — permissiv, keine Schema-Anpassung nötig). Einziger Rest (dev-1-Zone, rein
kosmetisch): `produkt/haut/KONZEPT.md:45` "lossy-transparent" → "dokumentiert-transparent" (Doku).
