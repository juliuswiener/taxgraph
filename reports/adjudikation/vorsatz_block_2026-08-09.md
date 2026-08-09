# Vorsatz-Block — 9 checkESt-Plausibilitätsfehler geschlossen

Auftrag (team-lead): 9 von 18 checkESt-Fehlern auf dem Produkt-XML stammen aus einem komplett
fehlenden `<Vorsatz>`-Block. Scope strikt `produkt/import/elster_xml.py` +
`tests/test_elster_xml.py`. Diese Datei ist der Nachweis: Befund, Implementierung, Messung,
Mutationsbeweis.

## Befund: Vorsatz ist kein Kz

`Vorsatz` und seine Kinder (Unterfallart, Vorgang, StNr, Zeitraum, AbsName, AbsStr, AbsPlz, AbsOrt,
Copyright, OrdNrArt, Rueckuebermittlung/Bescheid) tragen kein `E\d{7}`-Namensmuster. `kz_pfade()`
(`produkt/import/elster_xml.py:56`) indiziert nur Kz — dieser Block taucht dort nie auf, egal was
in `deklaration` steht. Unabhängig bestätigt durch den parallelen Report
`reports/adjudikation/stammdaten_inventur_2026-08-09.md:33-39` ("braucht einen separaten
Schreibpfad in `erzeuge_xml()`, analog zu `_transfer_header()`"). Deshalb: eigene Funktion
`_vorsatz()`, von Hand gebaut, genau wie `_transfer_header()` (`elster_xml.py:241`).

## Schema-Fakten (nichts geraten)

Primärquelle amtliches Schema:
`~/02_Software/eric/doc_extract/ERiC-44.2.4.0/Dokumentation/Datenarten/ElsterErklaerung/ESt/Schema/2025/E10-2025.xsd`
(NICHT `elster11_E10_2025_extern.xsd` wie in der ursprünglichen Briefing-Angabe — dort steht kein
`Vorsatz`; das ist nur ein Wrapper/Import-File. `E10-2025.xsd` im selben Verzeichnis ist die
tatsächliche Quelle).

| Fakt | Fundstelle | Wert |
|---|---|---|
| Position | `E10-2025.xsd:8403` | letztes Kind von `<E10>`, `minOccurs="0" maxOccurs="1"` |
| Kindreihenfolge | `Vorsatz_67907_CType`, `E10-2025.xsd:25286-25360` | Unterfallart, Vorgang, StNr, (Ordnungsbegriff, ID, IDEhefrau — unbenutzt), Zeitraum, AbsName, AbsStr, AbsPlz, AbsOrt, Copyright, (TeleNummer — unbenutzt), OrdNrArt, Rueckuebermittlung→Bescheid |
| Unterfallart | `E10-2025.xsd:8079-8087` (Pattern-Typ) | nur `"10"` zulässig |
| Vorgang | `Enum_Vorsatz_Vorgang_CType`, `E10-2025.xsd:7466-7479` | `"01"`=Veranlagung / `"04"`=Veranlagung+Vorauszahlungsfestsetzung |
| OrdNrArt | `E10-2025.xsd:7974-7982` (Pattern-Typ) | `"S"` oder `"O"` |
| StNr | `SteuernummerBaseCType`, `E10-2025.xsd:1779-1788` | Pattern `([0-9]{4})0[0-9]{8}` — 4-stellige Bundesfinanzamtsnummer + `0` + 8 Stellen |
| Bescheid | `JaNein12BaseCType`, `E10-2025.xsd:1634-1649` | `"1"`=Ja / `"2"`=Nein |

Referenz (`elster/submission/testfall_est2025_minimal.xml:108-122`, validiert rc=0) als Wertebeleg
für den Normalfall: `Unterfallart=10`, `Vorgang=01`, `OrdNrArt=S`, `Bescheid=2`.

## Implementierung

`_vorsatz()` neu, direkt vor `_transfer_header()` (`produkt/import/elster_xml.py:242-298`). Lebt im
`ns_e10`-Namespace, nicht im Elster-Rahmen-Namespace (Referenz-XML Zeile 108: kein eigenes Präfix,
Kind von `<E10>`). Vollständiger Docstring mit Quellenangabe pro Konstante — Details im Diff.

`erzeuge_xml()` bekommt ein neues Keyword `abgabefaehig: bool = False` plus fünf optionale
Absender-Parameter (`absender_name`, `absender_strasse`, `absender_plz`, `absender_ort`,
`absender_steuernummer`, alle `str | None = None`).

**Bauform-Entscheidung (Absender-Naht):** `abgabefaehig`-Flag statt z.B. stillschweigend immer
Vorsatz anzuhängen, sobald `absender_*` gesetzt sind. Begründung:
1. **Rückwärtskompatibilität messbar erzwungen**: alle ~68 bestehenden `erzeuge_xml()`-Aufrufe in
   `tests/` rufen ohne `abgabefaehig` — Default `False` heißt, kein bestehender Test sieht je
   Vorsatz, ohne dass ich einen einzigen bestehenden Aufruf ändern musste. 20 vorbestehende Tests
   in `tests/test_elster_xml.py` weiterhin grün (siehe Suiten-Ergebnis unten).
2. **Fail-closed statt Rate-Default**: Vorsatz ist strukturell nicht deklarierbar (kein Kz) — ohne
   explizites Flag gäbe es keinen Weg, "Nutzer hat Vorsatz absichtlich weggelassen" von "Vorsatz
   fehlt aus Versehen" zu unterscheiden. Das Flag macht die Absicht explizit, und mit
   `abgabefaehig=True` bei fehlenden `absender_*` bricht der Aufruf mit `XmlFehler` ab, statt ein
   XML zu bauen, das checkESt/das Finanzamt später klaglos zurückweist.
3. **Ein zusätzlicher Parameter über die brief-genannten 4 hinaus**: `absender_steuernummer` war in
   der ursprünglichen Aufgabenstellung nicht als Case-Parameter gelistet (nur AbsName/AbsStr/
   AbsPlz/AbsOrt). Grund: gemessener Befund, siehe unten — ohne StNr ersetzt `OrdNrArt="S"` die 9
   Fehler nicht durch 0, sondern durch 2 andere. StNr ist deshalb genauso fail-closed Pflicht wie
   die anderen vier.

Zusätzlich: ein Präfix-Konsistenz-Check `absender_steuernummer[:4] != empfaenger_finanzamt` →
`XmlFehler`, ebenfalls ein gemessener (nicht aus dem XSD ableitbarer) Befund.

**Abweichung von der Referenz — bewusst, dokumentiert:** Referenz-XML setzt `<Copyright>ELSTER
</Copyright>`. Das ist die Signatur des Erstellers der amtlichen Beispieldatei, nicht "Hersteller
der Steuersoftware" (die eigentliche Feldbedeutung laut Schema-Doku, `E10-2025.xsd:25343-25346`,
max. 50 Zeichen freier String, kein Enum). TaxGraph trägt diesen Sachverhalt bereits im
`TransferHeader` über den vorhandenen `datenlieferant`-Parameter (Default `"TaxGraph"`) — `_vorsatz()`
verwendet denselben Wert statt den Referenz-Literal zu kopieren. Explizit im Docstring vermerkt.

## Messung: checkESt vor/nach

Kommando (HERSTELLER_ID nie im Klartext):
```
cd /home/julius/00_projects/168_TaxGraph/taxgraph
set -a; . ./.env; set +a
export ELSTER_HERSTELLER_ID ERIC_DIR="$HOME/02_Software/eric"
python3 <scratch>/durchstich_checkest_baseline.py 2>&1 | sed "s/$ELSTER_HERSTELLER_ID/<ID>/g"   # VORHER, kein abgabefaehig
python3 <scratch>/durchstich_checkest.py          2>&1 | sed "s/$ELSTER_HERSTELLER_ID/<ID>/g"   # NACHHER, abgabefaehig=True + Absender
```

| Fall | vorher (Fehler) | nachher (Fehler) | Delta |
|---|---|---|---|
| Einzelveranlagung 60.000 EUR | 18 | 9 | **-9** |
| Zusammenveranlagung 2×50.000 EUR | 24 | 15 | **-9** |

Beide Fälle: rc bleibt `610001002` (Plausibilitätsfehler) — erwartungsgemäß, denn der Fall hat noch
keine Stammdaten (Name, Adresse, Religion, Bankverbindung, Anlage-N-Steuerklasse etc., siehe
`stammdaten_inventur_2026-08-09.md`), und diese Fehler liegen außerhalb meines Scopes.

Die 9 verschwundenen Fehler sind exakt die 9 aus der Aufgabenstellung:
```
Der Absendername muss im Feld $/Vorsatz[1]/AbsName[1]$ angegeben werden.
Die Straße des Absenders muss im Feld $/Vorsatz[1]/AbsStr[1]$ angegeben werden.
Der Ort des Absenders muss im Feld $/Vorsatz[1]/AbsOrt[1]$ angegeben werden.
Die Unterfallart muss im Feld $/Vorsatz[1]/Unterfallart[1]$ angegeben werden.
Der Vorgang muss im Feld $/Vorsatz[1]/Vorgang[1]$ angegeben werden.
Der Zeitraum muss im Feld $/Vorsatz[1]/Zeitraum[1]$ angegeben werden.
Das Copyright muss im Feld $/Vorsatz[1]/Copyright[1]$ angegeben werden.
Die Art der Ordnungsnummer muss im Feld $/Vorsatz[1]/OrdNrArt[1]$ angegeben werden.
Im Feld $/Vorsatz[1]/Rueckuebermittlung[1]/Bescheid[1]$ ist anzugeben, ob die Bereitstellung ...
```
Die verbleibenden 9 (einzel) bzw. 15 (zusammen) Fehlermeldungen sind identisch vor und nach der
Änderung (Namen/Adresse/Religion/Bankverbindung/Anlage-N-Steuerklasse/KAP-Pflichtangabe) — kein
neuer Fehler eingeführt, keine Seiteneffekte gemessen.

### Zwischenbefund während der Messung: OrdNrArt="S" ohne StNr

Erster Versuch (Absender ohne `absender_steuernummer`, `StNr` weggelassen) ergab **11** Fehler
statt der erwarteten 9 — checkESt akzeptierte das nicht klaglos, sondern ersetzte die 9
Original-Fehler durch 2 ANDERE:
- "Es wurde ... angegeben, dass die Steuererklärung mit einer vorhandenen Steuernummer abgegeben
  wird [...]"
- "Die Bundesfinanzamtsnummer und die ersten 4 Stellen der Steuernummer unterscheiden sich."

`OrdNrArt="S"` (Ordnung nach Steuernummer) verlangt also eine `StNr`, deren erste 4 Stellen mit der
Finanzamtsnummer übereinstimmen — eine reine Plausi-Regel, aus dem XSD allein nicht ableitbar. Mit
`absender_steuernummer="9181081508155"` (Präfix `9181` = Default `empfaenger_finanzamt`) verschwand
dieser Nebeneffekt und die Messung traf exakt -9/-9. Das ist der Grund für den fünften
Absender-Parameter und den Präfix-Check in `erzeuge_xml()` — kein Scope-Creep, sondern eine
gemessene Notwendigkeit, ohne die das Flag falsche Fehlerfreiheit vorgetäuscht hätte.

## XSD-Validierung

`test_abgabefaehiges_xml_ist_xsd_valide` (neu, `@braucht_xsd`) lässt das mit `abgabefaehig=True`
erzeugte XML durch `elster/submission/validate_xsd.py:validate()` (xmllint gegen das amtliche
Schema) laufen — grün.

## Tests (8 neu, Datei `tests/test_elster_xml.py`)

Alle im Stil der bestehenden "ERiC-Rahmengate"-Sektion:

1. `test_ohne_abgabefaehig_gibt_es_keinen_vorsatz_block` — Default bleibt unverändert.
2. `test_abgabefaehig_haengt_vorsatz_mit_allen_pflichtfeldern_an` — jedes Kind mit korrektem Wert.
3. `test_vorsatz_kinder_in_schema_reihenfolge` — xs:sequence-Reihenfolge, auf den Vorsatz-Teilstring
   eingeschränkt (sonst Kollision mit dem zweiten `<Vorgang>` im TransferHeader).
4. `test_vorsatz_ist_das_letzte_kind_von_e10` — Position nach allen Kz-Containern.
5. `test_abgabefaehig_ohne_absender_ist_fail_closed` — Crash statt stiller Null.
6. `test_abgabefaehig_nennt_jedes_fehlende_absender_feld` — Fehlermeldung nennt das konkrete Feld.
7. `test_steuernummer_praefix_muss_zur_finanzamtsnummer_passen` — der gemessene Befund oben, jetzt
   als Regressionsgate.
8. `test_abgabefaehiges_xml_ist_xsd_valide` — amtliches Schema, s.o.

Ergebnis: `python3 -m pytest -q tests/test_elster_xml.py` → **28 passed** (20 vorbestehend + 8 neu).

## Mutationsbeweis (jeder neue Test wurde real rot gefahren)

Vorgehen: Backup der sauberen Datei, gezielte Mutation, `pytest tests/test_elster_xml.py`, Rückbau,
grün bestätigt. Fünf Mutationen:

| # | Mutation | Rot (erwartet) | Ergebnis |
|---|---|---|---|
| 1 | `e10.append(_vorsatz(...))` auskommentiert | Tests 2-4 | 3 failed, 25 passed ✓ |
| 2 | Copyright vor AbsPlz/AbsOrt verschoben (Reihenfolge kaputt) | Test 3 | 2 failed, 26 passed ✓ (traf zusätzlich Test 8, weil die verdrehte Reihenfolge auch XSD-ungültig ist) |
| 3 | Fail-closed-Check `if fehlend_absender:` → `if False and fehlend_absender:` | Tests 5-6 | 2 failed, 26 passed ✓ (Test 5 crasht mit `TypeError` statt `XmlFehler`, weil die Ausführung bis zum Präfix-Check mit `absender_steuernummer=None` durchläuft — auch das ist "rot", nur über einen anderen Fehlertyp) |
| 4 | Präfix-Check `if absender_steuernummer[:4] != empfaenger_finanzamt:` → `if False and ...` | Test 7 | 1 failed, 27 passed ✓ |
| 5 | `if abgabefaehig:` → `if True:` (Flag ignoriert) | Test 1 | 14 failed, 14 passed ✓ (Kollateralschaden erwartet: fast jeder vorbestehende Aufruf ohne `absender_*` löst jetzt den Fail-closed-Pfad aus — Test 1 ist unter den 14, wie vorgesehen) |

Nach jeder Mutation zurückgesetzt (`diff` gegen Backup bestätigt identisch), letzter Lauf vor Commit:
`28 passed`.

## Vollständige Suite

`python3 -m pytest -q` (Baseline nach `cebb228`: 1668 passed, 4 skipped, 0 failed), gelaufen auf
HEAD `4966726` (weitere Commits paralleler Worker seit der Baseline) vor meinem eigenen Commit:

```
1685 passed, 4 skipped, 1 warning in 214.89s
```

0 failed, 0 unerwartet rot. Differenz zur Baseline (+17 passed) erklärt sich durch parallele
Commits anderer Worker im selben Zeitraum (8 davon meine eigenen neuen Tests), nicht durch
Regressionen. Die 4 Skips sind dieselben vorbestehenden (siehe `braucht_xsd`/plattformbedingte
Marker, nicht von dieser Änderung berührt). Die eine Warning ist eine vorbestehende
`StarletteDeprecationWarning` aus `tests/test_ui_backend.py`, unabhängig von dieser Änderung.
