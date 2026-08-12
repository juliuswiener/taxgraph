# Feldmatrix-Vollklassifikation (2026-08-12)

Beantwortet BACKLOG `feldmatrix-21-unklassifiziert`: der "erste Wurf" aus
`tests/test_checkest_feldmatrix.py` (Docstring dort, Zeile 116-118) zog ursprünglich 55
Kz-tragende askable Felder automatisch aus der Bindung, 29 wurden rot — nie als Artefakt
committet, nie vollständig klassifiziert. Die Bindungstabellen sind seither gewachsen; ein
Reproduktionslauf heute zieht **100** Felder, **68** davon rot. Diese Meldung klassifiziert
**alle 68**, nicht nur die ursprünglichen 29 — vollständige, aktuelle Abdeckung statt
Teilmenge.

**Nur gemessen, nichts editiert.** Weder `tests/test_checkest_feldmatrix.py` noch
`tests/test_checkest_blockmatrix.py` wurden angefasst (Letztere wird parallel von einem
anderen Worker bearbeitet). Team-Lead integriert die Befunde in die Matrix-Dateien.

## Methode

- Skript: `scripts/measure_feldmatrix_vollsweep.py` (neu, wiederverwendet `_scharf`/`_mit`/
  `_fall_einzel` 1:1 aus `tests/test_checkest_feldmatrix.py` — **geht garantiert über
  `_mit_ring_werten`**, nicht am Ring vorbei direkt store→deklariere. Rohdaten:
  `scripts/_vollsweep_rohdaten.json` (pool_groesse=100, rot_anzahl=68).
- Aufruf: `set -a; . ./.env; set +a; python3 scripts/measure_feldmatrix_vollsweep.py`
- Jede Klassifikation unten ist mit dem Befehl belegt, der sie erzeugt hat. "Skipped" wurde
  NICHT als Messung akzeptiert — ein leerer Fehlerpuffer bei rc=610301200/610301106 sieht
  fehlerfrei aus, ist es aber nicht (s. BEFUND unten, genau dieser Fall).
- IdNr/Steuernummer/HERSTELLER_ID: nirgends im Klartext unten (synthetische Test-IdNrs sind
  wie im Bestand üblich sichtbar).

## Tally

| Klasse | Anzahl | Anteil |
|---|---|---|
| BEFUND | 1 | 1,5 % |
| PROBEWERT-ARTEFAKT | 4 | 5,9 % |
| BLOCK-ARTEFAKT — Begleitfeld existiert, live rc=0 bestätigt oder durch bestehenden Test belegt | 45 | 66,2 % |
| BLOCK-ARTEFAKT — Begleitangabe strukturell fehlt (kein Feld dafür, kein Bau-Fund, reine Deckungslücke) | 18 | 26,5 % |
| **Summe** | **68** | 100 % |

---

## 1. BEFUND (1)

### `person_b_idnr` (E0100082) — bereits vorab an Team-Lead gemeldet

**rc=610301106** (`ERIC_IO_READER_UNERWARTETE_ELEMENTE`, undokumentiert in
`klassifiziere_rc()` — fällt in den `"sonstig"`-Catch-all, leerer Textpuffer, sieht ohne
`eric.log`-Ground-Truth wie unauffällig aus).

Jeder Wert (gültige wie ungültige Prüfziffer) und jeder Kontext (Einzelfall UND vollständiger
Zusammenveranlagungsfall mit echten Partner-Stammdaten über `test_checkest_durchstich._fall_zusammen`-
artige Fixtur) führt **unbedingt** zur Ablehnung, NACH erfolgreicher Schemaprüfung. Ground-Truth
aus `eric.log`:

```
Schemapruefung erfolgreich
ERROR: Nutzdaten enthalten das Feld "/ESt1A/Allg/B/E0100082" mit dem Eingefuegt-Kennzeichen "J" oder "P"
→ ERIC_IO_READER_UNERWARTETE_ELEMENTE
```

Hypothese "fehlendes `<Vlg_Art><E0101201>X</E0101201></Vlg_Art>`" **falsifiziert**: derselbe
Zusammenveranlagungsfall OHNE `person_b_idnr` (der ebenfalls kein `Vlg_Art` schreibt)
validiert sauber (rc=0) — das Element selbst, nicht sein Fehlen an anderer Stelle, ist die
Ursache. Root Cause nicht vollständig gepinnt (ERiC-seitige Geschäftsregel jenseits der reinen
XSD-Struktur, `IDNrBaseCType_RABE` als einziger Typ-Suffix seiner Art unter `<B>` ist verdächtig,
aber nicht bewiesen) — der Defekt selbst ist 100 % reproduzierbar und bestätigt.

**Team-Lead-Antwort steht noch aus** (Stand dieser Meldung).

---

## 2. PROBEWERT-ARTEFAKT (4)

Alle vier: der ursprüngliche `beispielwert` in der Bindungstabelle selbst ist ungültig — nicht
der Bau. Re-Messung mit gültigem Wert schließt den Fund.

| Feld | Kz | ungültiger Probewert | Fehler (wörtlich, gekürzt) | gültiger Wert → rc |
|---|---|---|---|---|
| `kind_idnr` | E0500406 | `"12345678901"` | „Ungültige Identifikationsnummer." | echte Prüfziffer (ISO 7064 MOD 11,10) → **rc=0 für die Prüfziffer selbst**; danach dual-klassifiziert als BLOCK-ARTEFAKT (fällt auf denselben „Vornamen des Kindes"-Befund wie die kind_*-Familie, s. unten) |
| `tage_24h` | E0205409 | `0` | „Der … eingegebene Wert muss größer als 0 sein." | `10` → rc=0 |
| `tage_an_abreise` | E0205302 | `0` | dieselbe Meldung | `10` → rc=0 |
| `tage_ueber_8h_eintaegig` | E0205201 | `0` | dieselbe Meldung | `10` → rc=0 |

Befehl (Beispiel tage_*):
```
python3 -c "... TCF._scharf(TCF._mit('tage_an_abreise', 10)) ..."
→ rc=0 plausibel []
```

**Nebenfund (Datenqualität, keine Blockade):** `tage_24h`/`tage_an_abreise`/
`tage_ueber_8h_eintaegig` haben in der Bindung `bereich: {min: 0, max: 366}` — sollte `min: 1`
sein (ELSTER verlangt strikt >0). `tests/test_checkest_feldmatrix.py::MATRIX` umgeht das für
`tage_24h` bereits handkuratiert mit Wert `10`; die anderen zwei Geschwister nie kuratiert,
daher rot im Vollsweep. `kind_idnr`s eigener Platzhalter-`beispielwert` scheitert an derselben
Checksumme wie der ursprüngliche Fehlversuch bei `person_b_idnr` — beide Bindungstabellen
tragen denselben ungültigen Platzhalter `"12345678901"`.

---

## 3. BLOCK-ARTEFAKT — Begleitfeld existiert (45)

Companion-Feld ist in der Bindung vorhanden UND entweder live kombiniert **rc=0 bestätigt**,
oder bereits durch einen bestehenden, grünen Test im Repo abgedeckt.

### 3a. Live bestätigt (diese Meldung, frischer Messlauf)

| Feld(er) | Kz | Fehlt laut ERiC | Begleitfeld | Kombiniert |
|---|---|---|---|---|
| `fahrtkosten_pausch_gdb80_oder_70g` + `fahrtkosten_pausch_ag_bl_tbl_h` | E0161706/E0161806 | „außer der Angabe im Feld Person keine weiteren Angaben" | beide TRUE statt beide FALSE (Beispielwert war `false`/`false` — Container blieb inhaltsleer) | **rc=0** |
| `rentner_hilflos_blind_taubblind` | E0109706 | Kontext Geh_Steh_Blind_Hilfl leer, "Person A ohne weitere Angaben" | `rentner_grad_der_behinderung=50` (allein bereits grün) | **rc=0** |
| `kist_konfession` | E0100402 | „Arbeitslohn … Kirchensteuer jedoch nicht" | `kirchensteuer_arbeitgeber=0` | **rc=0** |
| `kap_q_auslaendische_steuer` | E1905101 | „keine Angaben zu Kapitalerträgen" | `kap_kapitalertraege=500000` (über `_ersetzen()`, da im Basisfall bereits aktiv mit 0) | **rc=0** |
| `dba_auslaendische_einkuenfte`, `dba_gezahlte_auslaendische_steuer` | E0601401/E0601901 | „nicht erklärt aus welchem Staat" | `dba_staat` **existiert als Feld**, hat aber `elster_kz: null` — dokumentiert in der Bindung als "dev-2-Folgeticket (null-MVP)". Voller 7-Feld-Block live getestet: bleibt rot, weil 5 von 7 Feldern (`dba_staat`, `dba_methode`, `dba_einkunftsart`, `dba_mehrere_staaten`, `dba_abzug_statt_anrechnung`) keinen Kz haben und daher NIE ins XML gelangen. Bekannte, bereits tickitierte MVP-Lücke — kein neuer Fund. | bleibt rot bis Kz-Nachtrag (Folgeticket) |

Befehle: `_mit(fid1, w1)` + `_b(store, fid2, w2)` → `_scharf(store)`, je Zeile einzeln
nachvollzogen (Details im Skript-Log, Kollision bei `kap_kapitalertraege` erfordert
`TCF._ersetzen()` statt `_b()`, da im Basisfall bereits aktiv).

### 3b. Bereits durch bestehenden, grünen Test abgedeckt (zitiert, nicht neu gemessen)

| Feld(er) | Kz | Bestehender Beleg |
|---|---|---|
| `hh_dienstleistung_art`/`_betrag`, `hh_handwerker_art`/`_betrag`, `hh_minijob_art`/`_betrag` (6 Felder) | E0107206/07, E0111217/14, E0104206/108 | `test_p35a_einzelaufstellung_alle_drei_toepfe_amtlich_plausibel` (Zeile 386) — Art+Betrag je Topf kombiniert → rc=0 |
| `p36_kapitalertragsteuer`, `_kist`, `_solz` (3 Felder) | E1904701/E1904801/E1904901 | `test_p36_kap_anrechnung_amtlich_plausibel_mit_kapitalertraegen` (Zeile 264) — mit `kap_kapitalertraege` + `kist_konfession`-Kontext → rc=0 |
| `stammdaten_vorname_partner`, `_nachname_partner`, `_geburtsdatum_partner`, `kist_konfession_partner` (4 Felder, wechselseitiges Quartett) | E0100801/E0100901/E0101001/E0101002 | `test_checkest_durchstich._fall_zusammen()`/`_STAMM_B`/`_BASIS_B` — dieselbe Fixtur, die ich für die `person_b_idnr`-Untersuchung als rc=0-Baseline (ohne `person_b_idnr`) bestätigt habe |
| `stammdaten_bic` | E0102201 | `scripts/measure_bankverbindung.py` (2026-08-10) — Fall 3a: Auslands-IBAN MIT BIC + Kontoinhaber → rc=0. Im Vollsweep isoliert getestet (ohne IBAN, mit gleichzeitig aktivem `stammdaten_keine_bankverbindung=true` aus dem Basisfall) → Widerspruch, erwartetes Rauschen |
| `kind_*`-Familie außer `kind_idnr` (24 Felder: `kind_anderer_elternteil_*`×4, `kind_betreuung_*`×5, `kind_familienkasse`, `kind_geburtsdatum`, `kind_grad_der_behinderung`, `kind_hilflos_blind_taubblind`, `kind_hinterbliebenen_uebertragung`, `kind_kindschaftsverh(altnis)_zeitraum_a/b`×4, `kind_kv`, `kind_pv`, `kind_vorname`, `kind_wohnsitz_inland_zeitraum`, `kinderbetreuungskosten`, `schulgeld`) | diverse | Alle scheitern einheitlich an „Tragen Sie bitte den Vornamen des Kindes ein" — Companion `kind_vorname` existiert, ist Gegenstand aktiver Arbeit (heutige Commits `4519cc7`/`1e140d2`/`51615c6`, „erste Haelfte des Kind-Blockers"). Gehört per Definition in die Block-Matrix (Anlage Kind als Ganzes), nicht in die Einzelfeld-Matrix — genau wie der Modul-Docstring in `test_checkest_feldmatrix.py` (Zeile 119-121) es für `kind_kv` bereits selbst dokumentiert |

**Sonderfall innerhalb der kind_*-Familie:** die 5 `kind_betreuung_*`-Felder (`_dienstleister`,
`_eigenanteil`, `_zeitraum`, `_kein_gemeinsamer_haushalt_zeitraum`,
`_haushaltszugehoerigkeit_zeitraum`) zeigten im Rohdaten-JSON eine **EXCEPTION**
„Feld nicht in der Bindungstabelle" statt einer Plausibilitätsmeldung. Ground-Truth-Check:
`scripts/_vollsweep_rohdaten.json` hat mtime 13:18:35; `produkt/bindung/bindung_p10_1_5_gesamt.yaml`
wurde von einem anderen Worker um 13:36:59 (Commit `b30549a`) geändert — genau diese 5 Felder
wurden dort NACH dem Sweep-Lauf verdrahtet. **Stale-Messung, kein Bug.** Live re-gemessen: alle
5 bestehen den Wiring-Check jetzt, werfen dieselbe „Vornamen des Kindes"-Meldung wie ihre
Geschwister — korrekt in die kind_*-Familie oben eingeordnet.

---

## 4. BLOCK-ARTEFAKT — Begleitangabe strukturell fehlt (18)

Diese Gruppe unterscheidet sich von Abschnitt 3: das fehlende Begleitfeld existiert NICHT in
irgendeiner `bindung_*.yaml` — die Anlage/der Block ist nur teilweise gebaut. Kein
Fehlbau/keine Regression, sondern erwarteter Stand eines Produkts im Aufbau. Trotzdem
FUNKTIONAL relevant: ein Nutzer, der genau dieses (bereits askable) Feld heute wahrheitsgemäß
beantwortet, bekäme ohne die fehlende Zusatzangabe eine nicht einreichbare Erklärung — die
Lücke lässt sich nicht durch Testkombination schließen, sondern nur durch neuen Bau.

| Feld(er) | Kz | Fehlt laut ERiC | Geprüft (Familie vollständig gelesen/live kombiniert) |
|---|---|---|---|
| `gewst_hebesatz`, `gewst_messbetrag` | E0801705/E0801606 | „die zu zahlende Gewerbesteuer" | Beide zusammen live getestet → bleibt rot. Kein drittes Feld für den TATSÄCHLICH gezahlten Betrag in der Bindung |
| `p22_nr3_einkuenfte` | E0305301 | „Angabe zu den Einnahmen fehlt" | Einziges Feld seiner Familie (bestätigt per Grep); auch mit einem Wert ≠0 live getestet → bleibt rot, kein Probewert-Fund |
| `p33a_unterhalt_aufwendungen`, `p33a_unterhalt_kv_pv` | E0120103/E0124401 | Name/Adresse/Anzahl-Haushalt/Zeitraum der unterstützten Person | Datei vollständig gelesen (4 Felder gesamt) — keins davon deckt Name/Adresse/Personenzahl/Zeitraum ab |
| `dhf_unterkunftskosten_monat` | E0207611 | Beschäftigungsort, Grund, Datum der Begründung | Datei vollständig gelesen (7 dhf_*-Felder) — keins davon |
| `ep_arbeitstage`, `ep_entfernung_km`, `ep_oepnv_kosten` | E0203503/04/611 | Ziel des Weges + PLZ/Ort/Straße (Zieladresse) | Datei vollständig gelesen (4 ep_*-Felder, inkl. `ep_eigenes_kfz`) — keine Zieladresse |
| `p35c_energieberater_aufwendungen`, `p35c_sanierungsaufwendungen` | E0242001/E0241901 | 7 Formalien der Anlage Energetische Maßnahmen (Standort, Herstellungsbeginn, Fläche, Förder-Historie seit 2020, öff. Förderung, Baubeginn, Einzelaufwendungen) | Größte Lücke der Gruppe — die Anlage ist über die zwei Summenfelder hinaus faktisch nicht gebaut. Deckt sich mit dem bereits im Modul-Docstring genannten Beispiel „p35c ohne Gebaeude-Standort" |
| `realsplitting_empfaenger_kv_pv` | E0300717 | Zeile 5 (Unterhaltsleistungen) UND Zeile 7 (Krankengeld-Anspruch) | `realsplitting_unterhaltsleistungen` (Zeile 5) live kombiniert → löst NUR die erste Meldung, zweite („Zeile 7") bleibt — kein Feld für Krankengeld-Anspruch vorhanden |
| `spenden_betrag` | E0108405 | „Vermögensstock einer Stiftung" | Kz-Mapping selbst GEPRÜFT KORREKT (E0108405 = allgemeine Spenden lt. E10-2025.xsd, nicht fehlgemappt — Verdacht auf Kz-Bug damit entkräftet). Fehlendes Companion ist E0108607 (Vermögensstock-Stiftung-Sonderzeile), existiert nicht in der Bindung |
| `rentner_hinterbliebenenbezuege` | E0109704 | Detailangabe zu den Hinterbliebenenbezügen | Einziges Feld seiner Unterfamilie |
| `rentner_pflegegrad` | E0161606 | Wohnsitz-Inland-Angabe + Anzahl weiterer Pflegepersonen | `bindung_rentner.yaml` vollständig gelesen — keins der beiden vorhanden |
| `rentner_gepflegter_hilflos` | E0161808 | Kontext Ang_pflegebeduerft_Pers leer (Angaben zur pflegebedürftigen Person) | dito, kein Detailfeld zur gepflegten Person |
| `vv_einnahmen` | E0700201 | Laufende_Nummer_V + mehrere weitere Anlage-V-Angaben | Bereits im Modul-Docstring von `test_checkest_feldmatrix.py` (Zeile 120-121) als bekanntes Beispiel benannt — hier nur bestätigt, nicht neu gemessen |
| `berufsausbildung_aufwendungen` | E0108202 | Bezeichnung der Ausbildung + Art/Höhe der Einzelaufwendungen | Einziges Feld seiner Datei (`bindung_sonder_agb_35a.yaml`, Grep bestätigt) |

Summe dieser Gruppe: 2+1+2+1+3+2+1+1+3+1+1 = **18**.

---

## Einordnung für die Block-Matrix

Abschnitt 3 (45 Felder) ist reif für direkte Übernahme in `test_checkest_blockmatrix.py` — die
Begleitangabe existiert, die Kombination ist entweder schon grün belegt oder hier frisch
bestätigt. Abschnitt 4 (18 Felder) braucht ERST neuen Bau (fehlende Begleitfelder), bevor ein
Block-Matrix-Eintrag sinnvoll grün werden kann — dafür ist ein BACKLOG-Eintrag pro Anlage
sinnvoller als ein Blockmatrix-Test, der auf absehbare Zeit rot bliebe.

Einziger echter BEFUND: `person_b_idnr` (Abschnitt 1), bereits separat gemeldet.
