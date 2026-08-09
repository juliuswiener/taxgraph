# Stammdaten-Felder: Name, Geburtsdatum, Adresse, Konfession, Bankverbindung, Art_Erkl

**Auftrag:** Team-Lead-Briefing 2026-08-09. Mindestumfang-Felder aus § 150 Abs. 2 AO in Store →
Bindung → `deklariere()` → ELSTER-XML verdrahten, um checkESt-Plausibilitätsfehler
("Kein Hauptvordruck", "Religion nicht angegeben", "Name/Vorname fehlt", "Adresse fehlt",
"Bankverbindung fehlt", "Art_Erkl fehlt") zu schließen.

**Eigene Dateien:** `produkt/store/`, `produkt/bindung/*.yaml`, `produkt/mapping/est_mapping.py`,
`produkt/traverser/traverser.py`, eigene Tests. Judgment-Call-Erweiterung: `produkt/haut/api_constants.py`
(reine Datenkonstanten, für test_g-Erreichbarkeit nötig) und `tests/test_bindungs_typ_vs_xsd_typ.py`
(fremdes Gate, aber durch meine Änderung real gebrochen — siehe unten).

Nicht angerührt (fremd, laut Briefing verboten): `produkt/import/elster_xml.py`,
`produkt/haut/api.py`, `tests/conftest.py`.

## checkESt: vorher/nachher (scharf gemessen, HERSTELLER_ID redigiert)

Kommando (Wert nie ausgegeben):
```
set -a; . ./.env; set +a
export ERIC_DIR="${ERIC_DIR:-$HOME/02_Software/eric}"
python3 <harness> 2>&1 | sed "s/$ELSTER_HERSTELLER_ID/<ID>/g"
```
Harness: `/tmp/.../scratchpad/durchstich_checkest.py` (Fall Einzelveranlagung 60.000 EUR,
Fall Zusammenveranlagung 2×50.000 EUR). Vorher-Messung: `messung_vorher_stammdaten.txt`
(vorherige Session, vor meinen Änderungen). Nachher-Messung: `messung_nachher_stammdaten.txt`
(dieser Lauf, Harness um alle 13 neuen Felder ergänzt + `person_a`→`deklaration`-Kosmetikbug im
Harness selbst gefixt).

| Fall | vorher | nachher |
|---|---|---|
| Einzelveranlagung 60.000 EUR | 9 Fehlermeldungen | 4 Fehlermeldungen |
| Zusammenveranlagung 2×50.000 EUR | 15 Fehlermeldungen | 8 Fehlermeldungen |

**Weggefallen (6 Meldungen, in beiden Fällen):**
- "Kein Hauptvordruck ESt 1 A vorhanden." (Art_Erkl → E0100001)
- "Auf dem Hauptvordruck ESt 1 A ist anzugeben, ob es sich um eine Einkommensteuererklärung..." (Art_Erkl)
- "Religion nicht angegeben oder kein gültiger Wert" (kist_konfession → E0100402)
- "Bitte geben Sie den Namen und Vornamen an" (stammdaten_nachname/vorname → E0100201/E0100301)
- "Bitte geben Sie die vollständige derzeitige Adresse an" (Straße/Hausnr./PLZ/Ort → E0101104/E0101206/E0100601/E0100602)
- "Bitte geben Sie Ihre Bankverbindungsdaten an oder erklären Sie..." (stammdaten_keine_bankverbindung → E0102002)

**Nebenbefund (indirekt, nicht im Mindestumfang erwartet, aber positiv):** im
Zusammenveranlagung-Fall verschwanden zusätzlich 3 widersprüchliche Meldungen
("Es handelt sich um eine Einzelveranlagung, daher darf für Person B keine Anlage N/KAP
ausgefüllt werden", "...Vorsorgeaufwand..."). Vermutung: ohne gesetztes Art_Erkl-Kz konnte
checkESt die Veranlagungsart nicht sauber bestimmen und hat intern auf Einzelveranlagung
geraten, was zu Widersprüchen mit den tatsächlich befüllten Person-B-Kz führte. Mit Art_Erkl
gesetzt erkennt checkESt die Zusammenveranlagung korrekt — die 3 Meldungen sind weg.

**Neu aufgetaucht (2 Meldungen, je 1× pro Person, nur bei gesetzter Konfession):**
"Arbeitslohn laut Lohnsteuerbescheinigung(en) Steuerklassen 1-5 angegeben, Kirchensteuer
jedoch nicht." Erwartete Folge der jetzt wahrheitsgemäßen Konfessions-Angabe: checkESt verlangt
konsequenterweise auch den auf der (fiktiven) Lohnsteuerbescheinigung ausgewiesenen
KiSt-Betrag. Das Erfassen dieses Betrags war nicht Teil des Mindestumfangs (nur
Konfession/Bekenntnis war gefordert) — kein Rückschritt, sondern eine neue, korrekte
Anforderung, die erst durch die wahrheitsgemäße Angabe sichtbar wird.

**Verbleibend, nicht meins (Task #7 Anlage N, Task #8 Anlage KAP):** KAP-Anlage-Pflichtangabe,
Steuerklasse/Lohnsteuer auf Anlage N. Nicht angerührt, wie im Briefing verlangt.

**Störfaktor offengelegt:** geteilter Working Tree, keine isolierten Worktrees — andere Worker
committen/ändern parallel. Die Messung spiegelt den Stand zum Messzeitpunkt, nicht garantiert
isoliert von fremden Änderungen.

## Gebaute Felder (14 Kz: 13 neu + 1 bestehendes Feld umgewidmet)

| feld_id | Kz | Person | Bindungsklasse |
|---|---|---|---|
| stammdaten_nachname | E0100201 | A | 1:1 |
| stammdaten_vorname | E0100301 | A | 1:1 |
| stammdaten_geburtsdatum | E0100401 | A | 1:1 (typ: datum) |
| stammdaten_strasse | E0101104 | A | 1:1 |
| stammdaten_hausnummer | E0101206 | A | 1:1 |
| stammdaten_plz | E0100601 | A | 1:1 |
| stammdaten_wohnort | E0100602 | A | 1:1 |
| stammdaten_keine_bankverbindung | E0102002 | A | 1:1 (bool) |
| stammdaten_art_est_erklaerung | E0100001 | A | 1:1 (bool) |
| stammdaten_nachname_partner | E0100901 | B | 1:1, eigenes Kz |
| stammdaten_vorname_partner | E0100801 | B | 1:1, eigenes Kz |
| stammdaten_geburtsdatum_partner | E0101001 | B | 1:1, eigenes Kz (typ: datum) |
| kist_konfession | E0100402 | A | **neu: Klasse i (WERTEKODIERUNG)**, vorher elster_kz=null |
| kist_konfession_partner | E0101002 | B | Klasse i (WERTEKODIERUNG) |

Person-B-Stammdaten tragen eigene Kz (analog `person_b_idnr`), kein PARTNER_INSTANZ-Reuse wie
bei den Einkommens-Kz — landen 1:1 in der Haupt-Deklaration, nicht im `person_b`-Bucket
(verifiziert in `test_person_b_stammdaten_direkt_in_deklaration`).

### Architektur-Entscheidung: signatur_slot vs. geltungsbedingung

`traverser.relevanz()` hängt bei `geltungsbedingung` das eigene Feld als Gate an seinen
`regel_id` — ist der bestätigte Wert `False`, wird die GESAMTE Pseudoregel-Scope
`ausgeschlossen`. Für `typ: bool`-Felder ist das ein echtes Risiko (Nutzer antwortet False auf
eine Stammdaten-Frage → ganzer Scope verschwindet). Deshalb: Person-A-Felder (inkl. der 2
Bool-Felder) nutzen `signatur_slot: <eigene feld_id>` (selbstreferenziell, wie
`bruttoarbeitslohn`, immer relevant). Person-B-Felder (alle typ text/datum/enum, nie bool)
nutzen `geltungsbedingung: beide_ehegatten_zusammen_veranlagt` (wie `person_b_idnr`) — inhaltlich
richtig (nur bei Zusammenveranlagung relevant) und mechanisch sicher (inert, da nie bool).

### Neue Dispatch-Klasse i: WERTEKODIERUNG

`est_mapping.py`: Laien-Enum → amtlicher XSD-Code, KEIN 1:1-Passthrough.
`kist_konfession`/`kist_konfession_partner`: `keine`→`11`, `evangelisch`→`02`,
`roemisch-katholisch`→`03` (verifiziert gegen `Enum_Religionsschluessel_ab_VZ_2014_3` und die
amtliche Referenz-XML `elster/submission/testfall_est2025_minimal.xml`). `andere` hat
**bewusst keinen Code** (~35 mögliche XSD-Werte, kein Raten) → fail-closed in
`nicht_deklariert`, nicht stillschweigend geraten.

**Offene Julius-Entscheidung:** welcher der ~35 übrigen Religionsschlüssel-Werte für "andere"
Antworten (jüdisch, altkatholisch, etc.) — aktuell fail-closed, kein Versand mit geratenem Code.

### Gate-Regression gefunden und selbst gefixt

`tests/test_bindungs_typ_vs_xsd_typ.py` (fremdes, aber allgemeines Gate: "jedes Kz fällt in
GENAU EINEN Prüfzweig, kein stiller Durchfall") brach durch meine Änderungen:
- `kist_konfession`/`_partner`: vorher `elster_kz: null` (ungeprüft), jetzt gesetzt → Test prüfte
  die Laien-Enum-Werte direkt gegen die XSD-enum (falsch, das ist ja der Witz von
  WERTEKODIERUNG) → Mismatch.
- `typ: datum` (neu, für die beiden Geburtsdatum-Felder) kannte der Test gar nicht →
  "unbekannter typ" Fehler.

Beides selbst gefixt (surgical, kein Downgrade der Prüfschärfe):
- Neuer Zweig `typ == "enum" and feld_id in EM.WERTEKODIERUNG`: prüft die ÜBERSETZTEN Codes
  (nicht die Laien-Werte) gegen die XSD-enum — bleibt scharf genug, um einen falschen Code
  (z.B. Tippfehler) zu fangen.
- Neuer Zweig `typ == "datum"`: prüft `beispielwert` gegen das XSD-Pattern (analog zum
  bestehenden `text`-Pattern-Zweig). `E0100401`/`E0101001` haben XSD-Typ
  `DatumTTpMMpJJJJBekanntBaseCType_RABE` mit TT.MM.JJJJ-Pattern — `beispielwert: "05.05.1955"`
  matcht.

Das ist keine "fremde Baustelle, die ich melde" — der Bruch stammt direkt aus meinen
Bindungsänderungen, also selbst behoben, wie von den Regeln verlangt.

## Bekannte, akzeptierte Einschränkung: zuruecklesen()/WERTEKODIERUNG

`est_mapping.zuruecklesen()` (Lab-N3-Round-Trip) baut `e_nach_feld` generisch aus allen Kz mit
nicht-null `elster_kz` und liefert bei Treffer den ROHEN Deklarationswert zurück. Für
`kist_konfession` (jetzt `elster_kz: E0100402`) hieße das: `zuruecklesen()` würde den amtlichen
Code ("03") liefern, nicht den Laien-Wert ("roemisch-katholisch") — `zuruecklesen()` kennt
Klasse i nicht. **Geprüft:** `kist_konfession`/`kist_konfession_partner` tauchen in KEINEM
bestehenden `zuruecklesen`-Test auf (`tests/test_est_mapping.py`, `tests/test_instanz_kern.py`
— je 0 Treffer beim Grep), keine bestehende Prüfung macht Exact-Dict-Equality auf
`rt["felder"]`. Kein heutiger Regressionsschaden. Architektonische Lücke bleibt bestehen, falls
`zuruecklesen()` je für Konfession gebraucht wird — nicht Teil dieses Auftrags, hier nur
dokumentiert.

## Weitere offene Entscheidungen (Julius)

- **Konfession "andere":** siehe oben, kein Code gewählt.
- **Bankverbindung:** nur Bool "keine Bankverbindung vorhanden" gebaut, kein IBAN/BIC-Erfassung
  (Mindestumfang-Entscheidung). Falls echte Bankverbindungen unterstützt werden sollen: eigener
  Baustein nötig.
- **Art_Erkl E0100001:** als askable Ja/Nein-Frage modelliert (`signatur_slot`,
  `fragetext_laie`). Da das Produkt ausschließlich ESt-Erklärungen erzeugt, könnte dies auch ein
  nicht-askable Konstantwert sein statt einer echten Interview-Frage — offene Modellierungsfrage.
  Die übrigen 4 Art_Erkl-Kz (E0100002/E0100003/E0100009/E0100302 — Arbeitnehmer-Sparzulage,
  Feststellung verbleibender Verlustvortrag etc.) bewusst nicht gebaut, feature-conditional.
- **Deferred, außerhalb Mindestumfang:** Titel, Hausnummerzusatz, "verheiratet seit"-Datum,
  Beruf Person B — keine Kz gebunden.

## Tests

Neu: `tests/test_stammdaten_felder.py` — 10 Tests, deckt alle 11 zuvor ungetesteten Kz
(E0100301, E0100402, E0100601, E0100602, E0100801, E0100901, E0101001, E0101002, E0101104,
E0101206, E0102002) via echtem `ast.Assert`.

**Mutations-Beweis (tatsächlich ausgeführt):** `WERTEKODIERUNG["kist_konfession"]["code"]["roemisch-katholisch"]`
von `"03"` auf `"04"` mutiert → `python3 -m pytest tests/test_stammdaten_felder.py -q` →
**3 failed, 7 passed** (`test_kist_konfession_wertekodierung[roemisch-katholisch-03]`,
`test_wertekodierung_code_typo_waechter`, `test_stammdaten_im_xml`). Zurückgesetzt →
`python3 -m pytest tests/test_stammdaten_felder.py tests/test_bindungstabelle.py -q` →
**36 passed**.

**Gate-Suite:** `python3 -m pytest tests/test_bindungstabelle.py -q` → 26 passed (test_g/test_j/test_m
jetzt grün, vorher 3 failed wegen fehlender Erreichbarkeits-/Test-Belege).

**Vollsuite:** `python3 -m pytest -q` → **1696 passed, 4 skipped, 0 failed** (220s). Vorher im
Zuge dieser Arbeit ein echter, selbstverursachter Regressions-Fail in
`test_bindungs_typ_vs_xsd_typ.py` — selbst untersucht und gefixt (siehe oben), danach grün.

## Geänderte/neue Dateien

- `produkt/mapping/est_mapping.py` — Klasse i (WERTEKODIERUNG) + Dispatch-Branch
- `produkt/bindung/bindung_an_gesamt.yaml` — 13 neue Bindungen
- `produkt/bindung/bindung_p51a_kirchensteuer.yaml` — kist_konfession elster_kz null→E0100402
- `produkt/haut/api_constants.py` — STAMMDATEN_FELDER(_PARTNER), Erreichbarkeits-Wiring
- `tests/test_bindungs_typ_vs_xsd_typ.py` — datum-Zweig + WERTEKODIERUNG-Ausnahme (Regressionsfix)
- `tests/test_stammdaten_felder.py` — neu, 10 Tests
