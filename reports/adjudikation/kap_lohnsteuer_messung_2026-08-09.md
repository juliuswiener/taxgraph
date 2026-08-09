# Messung: Anlage KAP (selbstverschuldet?) + Lohnsteuer-Normfrage (Fakten für Julius)

Auftrag team-lead, 2026-08-09/10. Reines Messen, keine Implementierung. Keine Datei
außer diesem Bericht geändert, nichts committet. Hersteller-ID nirgends im Klartext
(überall `<ID>`).

Umgebung:

```
cd /home/julius/00_projects/168_TaxGraph/taxgraph
set -a; . ./.env; set +a
export ERIC_DIR="$HOME/02_Software/eric"
```

ERiC 44.2.4.0, Datenart `ESt_2025`, Hersteller-ID `<ID>` (aus `.env`).

---

## Teil 1 — Ist die KAP-Meldung selbstverschuldet?

**Antwort: JA.** Die Meldung verschwindet vollständig, wenn die vier `kap_*`-Felder
gar nicht gesetzt werden. Sonst ändert sich NICHTS — gleiche 17 restlichen Meldungen,
gleicher rc. Das ist eine Deklarationspolitik-Frage (nicht deklarieren, was 0 ist),
kein Bau-Task gegen ELSTER.

### Messaufbau

Skript (Scratchpad, nicht im Repo):
`/tmp/claude-1000/-home-julius-00-projects-168-TaxGraph-taxgraph/db772487-ddbf-429b-a0e4-37a31085356e/scratchpad/kap_messung.py`

Basis = `_BASIS_A` aus `tests/test_checkest_durchstich.py:100-105` (Einzelveranlagung,
60.000 EUR Bruttoarbeitslohn), die vier `kap_kapitalertraege`/`kap_gewinn_aktien`/
`kap_verlust_aktien`/`kap_verlust_sonstige`-Zeilen als einzige Variable herausgezogen.
`kein_kap=True` bleibt in **beiden** Läufen gesetzt. Pfad: Store → `est_mapping.deklariere()`
→ `EX.erzeuge_xml()` (ohne `abgabefaehig`, wie im Durchstich-Test) → `CE.validate(xml, "ESt_2025")`.

Befehl:

```
python3 /tmp/.../scratchpad/kap_messung.py
```

### Ergebnis

| Szenario | `kap_*`-Felder | E19\*-Kz in `deklaration` | rc | Meldungen gesamt | KAP-Meldung dabei? |
|---|---|---|---|---|---|
| (a) | explizit `= 0` | `{'E1900901': 0, 'E1900701': 0, 'E1901301': 0, 'E1901201': 0}` | 610001002 | **18** | ja |
| (b) | weggelassen | `{}` | 610001002 | **17** | nein |

`vollstaendig=True` in beiden Läufen (Weglassen der Felder bricht `deklariere()` nicht —
`kein_kap=True` allein genügt, damit die Erklärung als vollständig gilt).

Diff der Fehlertexte zwischen (a) und (b) — **genau eine** Zeile, sonst identisch:

> Nur in (a): "Auf den Anlagen KAP und / oder KAP-BET wurden Kapitalerträge erklärt,
> die dem inländischen Steuerabzug unterlegen haben. Bitte geben Sie auf der Anlage KAP
> auch einen Grund für die Angabe der Kapitalerträge an (Antrag auf Günstigerprüfung,
> Antrag auf Überprüfung des Steuereinbehalts, Erklärung zur K[…]"
>
> Nur in (b): — (leer)

Alle übrigen 17 Meldungen (Vorsatz-Block, Hauptvordruck ESt 1 A, Stammdaten, Steuerklasse/
Lohnsteuer, Religion, Bankverbindung) sind in (a) und (b) wortgleich vorhanden — die
`kap_*`-Variation berührt sonst nichts.

**Mechanik:** `est_mapping.deklariere()` schreibt ein Kz, sobald das Feld im Snapshot
einen bestätigten Wert trägt — unabhängig vom `kein_kap`-Flag. Setzt der Test (bzw. ein
künftiger UI-Lauf) `kap_kapitalertraege=0` explizit, entsteht `E1900701=0` im XML.
ELSTERs Plausibilitätsprüfung sieht darin "Kapitalerträge erklärt" (auch bei Wert 0 als
irgend deklarierten Betrag unter Anlage KAP) und verlangt dafür einen Angabegrund-Kz.
Bleibt das Feld unbestätigt/ungesetzt, entsteht gar kein Kz, und die Prüfung greift nicht.

---

## Teil 2 — Fakten zur Lohnsteuer-Normfrage (KEINE Entscheidung)

Auslöser: checkESt verlangt in beiden Teil-1-Läufen (identisch, unabhängig von KAP):

> "Arbeitslohn laut Lohnsteuerbescheinigung(en) Steuerklassen 1 - 5 angegeben,
> Lohnsteuer jedoch nicht. Gegebenenfalls ist die Lohnsteuer mit dem Wert 0 zu
> erklären (Steuerpflichtige Person / Ehemann / Person A)."

(Quelle: Rohtext aus `CE.validate()`-Antwort, `<Text>`-Element, Lauf (a)/(b) oben.)

### 1) Exakter Wortlaut der bestehenden Adjudikation

`produkt/bindung/bindung_p36_abschlusszahlung.yaml:22-24`:

```yaml
    elster_kz: null
    kz_status: endgueltig
    elster_kz_grund: "ENDGÜLTIG kein E10-Kz (belegt 2026-08-05, nicht Backlog): E10 ist
    ein Erklärungs-Schema, die § 36-Anrechnung (festgesetzte ESt − LSt − Vorauszahlungen)
    ist ein Berechnungsschritt des FA-Systems. Die vier Kandidaten mit 'Lohnsteuer' im
    xs:documentation (E0200301–E0200304) liegen unter E10/N/ArbL/LStB_… = Anlage N
    Zeile 6, also Einkunftsermittlung — ein Mapping dorthin schriebe den Betrag doppelt
    zur eLStB. E10/WA_ESt/Anzur_Steu führt nur § 50a-Abzugsbeträge."
```

(Zeile 24 ist eine einzige YAML-Zeile; hier zum Lesen umgebrochen, Wortlaut unverändert.)

Zum Vergleich das Schwesterfeld `p36_vorauszahlungen`, `bindung_p36_abschlusszahlung.yaml:42`:

> "ENDGÜLTIG kein E10-Kz (belegt 2026-08-05, nicht Backlog): geleistete Vorauszahlungen
> sind dem FA aus der eigenen Festsetzung bekannt; die § 36-Anrechnung ist ein
> Berechnungsschritt des FA-Systems, kein Erklärungsfeld. […]"

Kontext-Kommentar am Dateikopf, `bindung_p36_abschlusszahlung.yaml:1-7`: "Reine
Post-Festsetzungs-Abrechnung: ändert NUR die Zahllast, nicht die ESt." Und in
`tests/test_nicht_deklariert_inventar.py:128-132`: die Umstufung von OFFEN nach
ENDGUELTIG am 2026-08-05 war "KEINE Kz-Arbeit, sondern eine korrigierte Einordnung: die
Zahl war vorher zu hoch, nicht die Lage besser geworden."

### 2) Gilt die Begründung nur für die Anrechnung, oder auch für die Erklärung selbst?

**Befund, kein Urteil:** Die Begründung nennt E0200301–E0200304 bereits **namentlich**
und explizit als Grund gegen ein Mapping ("schriebe den Betrag doppelt zur eLStB") —
sie deckt also nicht nur die Anrechnung (§ 36 Abs. 2 Nr. 2 EStG), sondern genau die
Frage, ob `p36_lohnsteuer` stattdessen als **Erklärungswert** unter Anlage N (Zeile 6)
gemappt werden sollte. Die dort genannte Begründung ist Doppel-Deklaration gegen die
eLStB — nicht "es gibt keinen Kz dafür" (den gibt es: E0200301 etc. existieren).

Gesetzestext-Befund `sources/gesetze-im-internet/estg_p36_2026-07-11.txt` (Volltext
gegrepped): § 36 EStG regelt ausschließlich Anrechnung (Abs. 2), Aufrundung (Abs. 3)
und Abschlusszahlung/Erstattung (Abs. 4) — **keine** Erwähnung einer separaten
Erklärungspflicht für Anlage N oder der eLStB. Die Norm selbst unterscheidet nicht
zwischen "Anrechnung" und "Erklärung"; sie regelt nur Ersteres. Die Erklärungspflicht
für Anlage N Zeile 6 (falls sie besteht) müsste aus einer anderen Quelle stammen —
**0 Treffer** für `§ 41b` oder `§ 25 EStG` als Datei unter `sources/` (`find sources
-iname "*p41b*" -o -iname "*_25_*"` → keine Treffer). **0 Treffer** für `eLStB` als
Suchbegriff in `sources/` (`grep -rn "eLStB" sources/` → leer). Der Begriff "eLStB" in
der Bindungs-Begründung ist damit ein produktinterner Kurzname, nicht durch eine
hinterlegte Quelle belegt.

Einziger inhaltlicher Treffer zum Thema in `sources/`: `sources/bfinv/anleitungen/
anl_n_2025.txt:15`: "Die Abgabe der Anlage N entfällt, wenn: [der Arbeitgeber die
Daten elektronisch übermittelt hat]" (Zeilen 10-15, amtliche Anleitung zur Anlage N
2025). Das stützt die Doppel-Deklarations-Sorge der Bindungsbegründung strukturell —
sagt aber nichts darüber, was passiert, wenn das Produkt **trotzdem** eine
Steuerklasse deklariert, ohne die zugehörige Lohnsteuer zu deklarieren (der hier
tatsächlich gemessene Fall).

### 3) Schema-Fakten zu E0200301/E0200401/E0200501

Kz-Lookup (`elster/kz_extract.py` gegen `E10-2025.html`) + Pfad-Lookup
(`EX.kz_pfade(2025)` gegen `produkt/import/elster_xml.py`):

| Kz | Sektion (Schema-Label) | Pfad (`kz_pfade`) | XSD `minOccurs` |
|---|---|---|---|
| E0200301 | LStB_1_5_Sum / "Lohnsteuer" | `E10/N/ArbL/LStB_1_5_Sum` | `0` |
| E0200401 | LStB_1_5_Sum / "Solidaritätszuschlag" | `E10/N/ArbL/LStB_1_5_Sum` | (nicht separat geprüft) |
| E0200501 | LStB_1_5_Sum / "Kirchensteuer des Arbeitnehmers" | `E10/N/ArbL/LStB_1_5_Sum` | (nicht separat geprüft) |

`minOccurs="0"` belegt an
`~/02_Software/eric/doc_extract/ERiC-44.2.4.0/.../ESt/Schema/2025/E10-2025.xsd:17677`:

```xml
<xs:element name="E0200301" type="DezimalzahlNichtNegOhneFuehrNull_MaxL15_MaxVK12_MinNK2_MaxNK2_CType_RABE" minOccurs="0" maxOccurs="1">
```

Das heißt: das **Schema selbst zwingt nichts**. Die checkESt-Meldung ("Steuerklasse
angegeben, Lohnsteuer jedoch nicht") ist eine **Plausibilitätsregel** von ERiC (Koexistenz
Steuerklasse↔Lohnsteuer), keine `minOccurs`-Pflicht. Der amtliche Hinweistext selbst
sagt "Gegebenenfalls ist die Lohnsteuer mit dem Wert 0 zu erklären" — das ist die
offizielle Formulierung dafür, dass eine 0-Deklaration die Plausibilitätsregel
befriedigt; ob das inhaltlich korrekt/gewollt ist (0 zu erklären, wenn real ein
Betrag > 0 einbehalten wurde, wie im Testfall mit `p36_lohnsteuer` unbefüllt), ist eine
Entscheidung, keine Schema-Tatsache.

Kein Treffer zu E0200301/E0200401/E0200501 in irgendeiner `produkt/bindung/*.yaml`
außer der ablehnenden Erwähnung in `bindung_p36_abschlusszahlung.yaml:24` (`grep -rn
"E0200301\|E0200401\|E0200501" produkt/bindung/*.yaml`) — die Kz sind aktuell komplett
ungebunden, nicht nur für `p36_lohnsteuer`.

Zusatzbefund (Teil der checkESt-Meldung, nicht im Auftrag, aber am selben Fall
gemessen): `grep -rn "steuerklasse" produkt/bindung/*.yaml` → **0 Treffer**. Der
Steuerklasse-Teil der Meldung ist also ebenfalls eine ungebundene Lücke (separates
BACKLOG-Item #7 "Anlage N: Steuerklasse + Lohnsteuer"), nicht Teil dieser Adjudikation.

### 4) Ist `p36_lohnsteuer` heute befüllt?

**Ja, aber nur für die Ring-Berechnung, nie für die Erklärung.**

- `produkt/import/vast_mapping.py:56`: automatischer Import-Kanal aus der elektronischen
  Lohnsteuerbescheinigung (VaSt-Belegart `LStB`) — Element `LSteuer` ("einbehaltene
  Lohnsteuer", Doku wörtlich aus der VaSt-XSD) mappt direkt auf `p36_lohnsteuer`.
- `bindung_p36_abschlusszahlung.yaml:17` (`askable: true`): zusätzlich manuell erfragbar
  ("Wie viel Lohnsteuer wurde im Jahr von Ihrem Arbeitgeber einbehalten und abgeführt?").
- `produkt/haut/api.py:1675`: `lst, vor = _best("p36_lohnsteuer"), _best("p36_vorauszahlungen")`
  — der Wert fließt in die Ring-Berechnung der Abschlusszahlung (§ 36 Abs. 4).
- `tests/test_differential_zone_d.py:134`/`:225`: Kommentar `# cent, null-kz ->
  nicht_deklariert` — der Klassifikationsstatus des Feldes ist im Testcode explizit als
  "wird berechnet, aber nicht in die XML-Deklaration geschrieben" dokumentiert.

Der Wert existiert im Snapshot (Import oder Laien-Eingabe), verlässt aber nie den Ring
Richtung XML — konsistent mit `elster_kz: null`.

---

## Bilanz für team-lead

**Teil 1:** Ja, die KAP-Meldung ist selbstverschuldet — sie verschwindet vollständig
(18→17, sonst identisch), sobald `kap_*`-Felder bei fehlender Kapitalertrag-Situation
nicht mit `0` deklariert, sondern schlicht weggelassen werden. Deklarationspolitik-
Entscheidung, kein ELSTER-Bau-Task.

**Teil 2 — Entscheidungsvorlage in 5 Sätzen:** Die bestehende Adjudikation (Zeile 24)
begründet `elster_kz: null` für `p36_lohnsteuer` explizit mit Doppel-Deklaration gegen
die eLStB, nicht mit Kz-Mangel — E0200301–304 existieren, `minOccurs=0`, unter
`N/ArbL/LStB_1_5_Sum` (Anlage N Zeile 6), strukturell getrennt vom § 36-Anrechnungspfad.
§ 36 EStG selbst regelt nur die Anrechnung, keine Erklärungspflicht; für eine
Erklärungspflicht-Norm zu Anlage N liegt in `sources/` nichts vor (0 Treffer § 41b,
§ 25, "eLStB"). `p36_lohnsteuer` ist heute befüllt (Import + Laien-Frage) und fließt in
die Ring-Berechnung, aber nie in die XML-Deklaration. Die amtliche Meldung selbst bietet
"0 zu erklären" als schema-verträglichen Ausweg an (kein `minOccurs`-Zwang), das sagt
aber nichts darüber, ob eine echte `p36_lohnsteuer`-Deklaration unter E0200301 korrekt
wäre oder tatsächlich zur eLStB doppelt liefe — das ist die offene Entscheidung.
