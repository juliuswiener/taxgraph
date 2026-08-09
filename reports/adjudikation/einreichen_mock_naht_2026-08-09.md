# Mock-Naht in `tests/test_einreichen.py` — Inventur, Falsch-Grün-Prüfung, Durchstich-Test

Auftrag von team-lead, 2026-08-09/10: die "Mock-Naht" schließen, über die
`tests/test_einreichen.py` fast durchgängig `erzeuge_xml` (und/oder `checkest_gate.validate`)
durch Platzhalter ersetzt. Dieselbe Bauart ließ `rc=610301200` (Namespace-Präfix-Ablehnung,
Fix `cebb228`) für das gesamte Projekt unentdeckt, bis `tests/test_checkest_durchstich.py`
(`969edd5`) sie auf FUNKTIONS-Ebene schloss. Auftrag hier: eine Ebene darüber, auf
ENDPUNKT-Ebene (`POST /fall/{id}/einreichen`).

Alle Zahlen unten stammen aus den genannten Befehlen. `ELSTER_HERSTELLER_ID` wird an keiner
Stelle ausgegeben — überall `<ID>`.

## Teil 1 — Inventur: alle 13 Tests in `test_einreichen.py`

Gelesen vollständig (`tests/test_einreichen.py`, 334 Zeilen, Stand HEAD `1ba90ee`). Jeder Test
klassifiziert nach: (a) reine Endpunkt-/Fehlerbehandlungslogik — Mock legitim, unverändert
lassen; (b) inhaltsabhängig — bräuchte echtes XML.

| # | Test | Was gemockt wird | Kategorie | Begründung |
|---|------|-------------------|-----------|------------|
| 1 | `test_leerer_fall_wird_nicht_eingereicht` | nichts | (a) | bricht vor dem XML-Bau (409 unvollständig) |
| 2 | `test_unbekannter_fall_404` | nichts | (a) | reine Routing-Prüfung |
| 3 | `test_antwort_behauptet_nie_versand` | nichts | (a) | bricht vor dem XML-Bau |
| 4 | `test_grund_ist_maschinenlesbar` | nichts | (a) | bricht vor dem XML-Bau |
| 5 | `test_unvollstaendig_nennt_die_offenen_felder` | nichts | (a) | bricht vor dem XML-Bau |
| 6 | `test_vorlaeufiges_feld_blockiert_einreichen` | nichts (Unit, kein HTTP) | (a) | bricht vor dem XML-Bau |
| 7 | `test_eric_fehlt_gibt_503_statt_crash` | `EX.erzeuge_xml`, `EM.deklariere`, `CE.validate` wirft `RuntimeError` | (a) | testet die Fehlerbehandlung, wenn die ERiC-Bibliothek selbst fehlt — `CE.validate` wirft, bevor irgendein XML-Inhalt ausgewertet würde; XML-Inhalt ist für diesen Testgegenstand irrelevant (team-lead nannte exakt dieses Beispiel) |
| 8 | `test_plausibilitaetsfehler_reicht_ericantwort_durch` | `EX`, `EM`, `CE.validate` gibt festen `rc` zurück | (a) | Prüfgegenstand ist das Response-Mapping bei gegebenem `rc` — der `rc` wird direkt injiziert, das XML dahinter ist nicht der Testgegenstand |
| 9 | `test_io_gate_rc_ist_nicht_gruen` | `EX`, `EM`, `CE.validate` → `(RC_IO_KEIN_TICKET, "")` | (a) | testet dieselbe Mapping-Logik für den Falsch-Grün-Fall isoliert vom echten ERiC-Aufruf — bleibt wertvoll als deterministischer Unit-Test, wird durch den neuen Durchstich-Test ergänzt, nicht ersetzt |
| 10 | `test_erfolg_meldet_plausibel_aber_nicht_eingereicht` | `EX`, `EM`, `CE.validate` → `(RC_OK, "")` | (a) | Erfolgs-Response-Mapping bei injiziertem `rc` |
| 11 | `test_xml_nicht_baubar_gibt_422` | `EX.erzeuge_xml` wirft `XmlFehler` | (a) | testet Fehlerbehandlung — `erzeuge_xml` MUSS hier werfen, Inhalt ist nicht der Testgegenstand |
| 12 | `test_einreichen_traegt_verpflegung_kuerzung_ein` | `EX.erzeuge_xml` fängt nur das `deklaration`-Dict ab (kein fester Platzhalter-Rückgabewert für den Input), `CE.validate` fest `RC_OK` | (a) | Prüfgegenstand ist der echte Pfad `_mit_ring_werten` → `EM.deklariere` (E0205508-Kürzungsbetrag) — `erzeuge_xml` wird bewusst nur als Abgriffspunkt benutzt, nicht um XML-Struktur/-Inhalt zu prüfen. Anderer Prüfgegenstand als die Mock-Naht |
| 13 | `test_einreichen_ohne_verpflegung_kein_kuerzung_kz` | wie #12 | (a) | wie #12, Inertheits-Gegenprobe |

**Ergebnis der Inventur: keiner der 13 Tests muss umgebaut werden.** Jeder mockt ehrlich genau
das, was er nicht prüfen will, und behauptet nichts anderes. Die Lücke ist nicht falsch
gescopte Tests, sondern die **Abwesenheit** eines Tests, der den Endpunkt einmal ganz ohne
Mock durchläuft — das bestätigt exakt die Diagnose aus `test_checkest_durchstich.py`s
Docstring ("Der Docstring von `test_checkest_gate.py` benennt das offen als 'Mock-Naht'") und
aus `test_checkest_gate.py`s eigenem Docstring ("`validate()` … wird in
`tests/test_einreichen.py` über die Mock-Naht abgedeckt").

## Teil 2 — Falsch-Grün-Regel: hält der Endpunkt sie ein?

Geprüft: `produkt/haut/api.py:einreichen()`, Zeilen 2405–2425 (Stand HEAD `1ba90ee`):

```python
try:
    import checkest_gate as CE
except ImportError as e:
    return 503, {..., "grund": "eric_nicht_verfuegbar", ...}
try:
    rc, antwort = CE.validate(xml, f"ESt_{vz}")
except (RuntimeError, OSError) as e:
    return 503, {..., "grund": "eric_nicht_verfuegbar", ...}

klasse = CE.klassifiziere_rc(rc)
basis = {..., "rc": rc, "klasse": klasse, ...}
if rc != CE.RC_OK:
    return 422, {**basis, "grund": "plausibilitaet_verletzt", "ericantwort": antwort, ...}
audit.append(...)
return 200, {**basis, "plausibel": True, ...}
```

**Befund: kein Falsch-Grün-Bug.** Die Erfolgs-Entscheidung hängt strikt an `rc != CE.RC_OK`
(Zeile 2419) — NICHT an der Länge/Leere von `antwort`. `rc=610301200`
(`RC_IO_KEIN_TICKET`, leerer Fehlerpuffer) fällt also in denselben `if`-Zweig wie jeder andere
`rc != 0` und bekommt 422, nie 200. Das deckt sich mit dem bereits bestehenden, mock-basierten
Test `test_io_gate_rc_ist_nicht_gruen` (Zeile 200–214) — den habe ich als echten Beleg genommen,
nicht nur als Existenzbeweis: er ist grün, WEIL Zeile 2419 tatsächlich so lautet, nicht trotzdem.
Kein Befund, der vor einem Fix hätte gemeldet werden müssen (Vorgabe Punkt 4) — es gibt hier
nichts zu reparieren.

**Kleine Nebenbeobachtung (kein Fix, nur vermerkt):** Das `grund`-Feld ist für ALLE `rc != 0`-Fälle
identisch `"plausibilitaet_verletzt"` — auch für `io_gate_nicht_geprueft` und
`hersteller_id_gesperrt`, die eigentlich "nicht geprüft" statt "geprüft und abgelehnt" bedeuten.
Der HTTP-Status bleibt in jedem Fall 422 (kein Falsch-Grün), aber wer nur auf `grund` schaut statt
auf `klasse`, bekommt eine ungenaue Fehlbezeichnung. Nicht angefasst — außerhalb des Auftrags,
und `klasse` transportiert die genaue Unterscheidung bereits korrekt.

## Teil 3 — Neuer Durchstich-Test: `tests/test_einreichen_durchstich.py`

Neue Datei, eine Ebene über `test_checkest_durchstich.py`: geht durch den echten HTTP-Server
(`server.make_server`), echten `einreichen()`-Code, echtes `erzeuge_xml`, echtes
`checkest_gate.validate` — nichts gemockt. Skip-Mechanik (`_hid()`/`braucht_eric`) 1:1 aus
`test_checkest_durchstich.py` übernommen (dupliziert, nicht importiert — beide Testdateien
bleiben eigenständig, wie im Repo üblich).

**Fixtur-Messung (2026-08-10):** die `_BASIS_A`-Feldmenge aus `test_checkest_durchstich.py`
reicht für den Endpunkt-Pfad NICHT direkt — `EM.deklariere` meldete die 4 KAP-Detailfelder
(`kap_kapitalertraege`, `kap_gewinn_aktien`, `kap_verlust_aktien`, `kap_verlust_sonstige`) als
"Feld nicht in der Bindungstabelle" (Anlage KAP, Task #8/#11 — bekannte offene Baustelle, nicht
Teil dieses Auftrags). Mit nur `kein_kap=True` (ohne die vier Detailfelder) wird die Deklaration
vollständig und der Lauf erreicht echtes XML und echtes checkESt.

Befehl und Ergebnis (Hersteller-ID aus `.env`, in der Befehlszeile nie ausgegeben):

```
$ python -m pytest tests/test_einreichen_durchstich.py -v
tests/test_einreichen_durchstich.py::test_einreichen_endpunkt_erreicht_die_amtliche_pruefung PASSED
1 passed in 1.08s
```

Gemessene echte Endpunkt-Antwort (Auszug, `ericantwort` gekürzt auf die Textliste):

```
rc     = 610001002   (RC_PLAUSIBILITAET)
klasse = "plausibilitaet_fehler"
status = 422
plausibel-Feld: nicht True (Endpunkt behauptet keinen Erfolg)
17 Fehlertexte, u.a.:
  - "Kein Hauptvordruck ESt 1 A vorhanden."
  - "Der Absendername muss im Feld $/Vorsatz[1]/AbsName[1]$ angegeben werden." (9× Vorsatz-Block)
  - "Die Steuerklasse wurde … auf der Anlage N nicht eingetragen …" (Task #7)
```

Das XML erreicht also nachweislich die amtliche Plausibilitätsprüfung, und der Endpunkt meldet
das ehrlich (422, `klasse="plausibilitaet_fehler"`, `plausibel` nicht gesetzt). `rc=0` wird
NICHT erwartet — Vorgabe Punkt 3 —, der Test prüft nur die qualitative Frage (kommt das XML
durch, meldet der Endpunkt es ehrlich), keine feste Fehlerzahl.

**Nebenbefund (informativ, nicht Teil des Auftrags):** die 9 Vorsatz-Block-Fehler, die
`test_checkest_durchstich.py`s `_pruefe()` seit `e365a37` per `abgabefaehig=True` +
`absender_*`-Parametern schließt, tauchen hier wieder auf — der echte `/einreichen`-Endpunkt
(`api.py:2398-2400`) übergibt `abgabefaehig` und `absender_*` nicht an `erzeuge_xml`. Die
Funktions-Ratsche (9/15 Restfehler) misst also einen saubereren Pfad, als der Live-Endpunkt ihn
heute tatsächlich fährt. Kein Falsch-Grün (der Endpunkt behauptet nirgends, dass diese Fehler
nicht da wären), aber erwähnenswert für die Priorisierung — team-lead zur Kenntnis, nicht selbst
angefasst.

## Teil 4 — Mutationsbeweis (Falsch-Grün-Kern, Endpunkt-Ebene)

Zeile 2419 in `produkt/haut/api.py` temporär ausgehebelt (`if rc != CE.RC_OK:` → `if False:`),
um zu zeigen, dass der neue Test einen echten Regress fängt, nicht nur zufällig grün ist:

```
$ git diff --stat produkt/haut/api.py   # vor Mutation: leer

# Mutation: if rc != CE.RC_OK:  ->  if False:  # MUTATION-PROOF
$ python -m pytest tests/test_einreichen_durchstich.py -v
FAILED tests/test_einreichen_durchstich.py::test_einreichen_endpunkt_erreicht_die_amtliche_pruefung
AssertionError: rc=610001002 != 0, aber der Endpunkt meldet Erfolg:
  {..., 'rc': 610001002, 'klasse': 'plausibilitaet_fehler', ..., 'plausibel': True, ...}
1 failed in 1.75s

# Rückbau (Edit rückgängig)
$ git diff --stat produkt/haut/api.py   # nach Rückbau: wieder leer
$ python -m pytest tests/test_einreichen_durchstich.py -v
1 passed in 1.08s
```

Skip-Mechanik geprüft (credential-freies CI simuliert):

```
$ env -u ELSTER_HERSTELLER_ID ERIC_DIR=/nonexistent python -m pytest tests/test_einreichen_durchstich.py -v
1 skipped in 0.52s
```

## Teil 5 — Volle Suite

Ausgangs-Commit: `1ba90ee`, mit meinen Änderungen (neue Datei
`tests/test_einreichen_durchstich.py` + dieser Report) im Arbeitsverzeichnis.

```
$ python -m pytest -q
1 failed, 1695 passed, 4 skipped in 213.95s (0:03:33)
FAILED tests/test_bindungs_typ_vs_xsd_typ.py::test_bindungs_typ_vs_xsd_typ
  - kist_konfession/kist_konfession_partner: enum-Werte vs. XSD-Kz E0100402/E0101002
  - stammdaten_geburtsdatum(_partner): unbekannter Bindungs-Typ 'datum'
```

Der eine Fehlschlag betrifft `tests/test_bindungs_typ_vs_xsd_typ.py` gegen
`produkt/bindung/bindung_an_gesamt.yaml`/`bindung_p51a_kirchensteuer.yaml`/
`produkt/mapping/est_mapping.py` — Dateien, die laut `git status` gerade von anderen Agents
(Task #7/#8, Anlage N/KAP, kist_konfession/stammdaten_geburtsdatum) parallel bearbeitet werden.
Mein Diff berührt weder diese Bindungsdateien noch `est_mapping.py` — `git diff --stat
produkt/haut/api.py` steht leer, die einzigen neuen Dateien von mir sind
`tests/test_einreichen_durchstich.py` und dieser Report. Der Fehlschlag ist nicht durch diese
Arbeit verursacht; nicht mein Task, wird team-lead zur Kenntnis gegeben, nicht selbst repariert.

Mein neuer Test läuft in diesem Lauf **im `passed`-Kontingent** (ERiC + Hersteller-ID im
Environment vorhanden, kein Skip).

## Zusammenfassung

- Inventur von 13 Tests in `test_einreichen.py`: alle 13 bleiben unverändert — jeder mockt genau
  das, was er nicht prüfen will, ehrlich und ohne falsche Behauptung. Die Lücke war die
  Abwesenheit eines echten Durchstichs, nicht falsch gescopte Mocks.
- Falsch-Grün-Regel am Endpunkt geprüft (`api.py:2416-2425`): hält, hängt strikt an `rc`, nicht
  an der Leere des Fehlerpuffers. Kein Befund, der vor Fix hätte gemeldet werden müssen.
- Neuer Test `tests/test_einreichen_durchstich.py::test_einreichen_endpunkt_erreicht_die_amtliche_pruefung`:
  echter HTTP-Durchstich, ECHTES XML, ECHTES checkESt, qualitativ (keine zweite Ratsche). Rot bei
  echter Mutation der Falsch-Grün-Weiche, grün nach Rückbau. Skip sauber ohne ERiC/ID.
- Nebenbefund: Live-Endpunkt fährt nicht den `abgabefaehig=True`-Pfad, den die Funktions-Ratsche
  in `test_checkest_durchstich.py` misst — 9 zusätzliche Vorsatz-Fehler tauchen hier auf. Kein
  Falsch-Grün, aber ein reales Delta zwischen gemessenem und live gefahrenem Pfad. Zur Kenntnis
  an team-lead, nicht selbst repariert (außerhalb des Auftrags).
- Volle Suite mit meinem Diff: 1 failed, 1695 passed, 4 skipped — der eine Fehlschlag
  (`test_bindungs_typ_vs_xsd_typ.py`) liegt an Bindungsdateien anderer, paralleler Tasks (#7/#8),
  nicht an dieser Arbeit. Mein neuer Durchstich-Test läuft grün im `passed`-Kontingent.
