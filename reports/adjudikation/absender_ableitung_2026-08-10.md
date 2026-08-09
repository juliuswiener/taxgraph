# Absender-Ableitung im Vorsatz-Block statt Pflicht-Parameter — 2026-08-10

## Auftrag

`erzeuge_xml()` verlangte bislang `absender_name/_strasse/_plz/_ort/_steuernummer` als eigene
Pflicht-Parameter für `abgabefaehig=True` — eine zweite Repräsentation derselben Angaben, die
bereits als Kz im Hauptvordruck ESt 1 A stehen. Der Live-Endpunkt `/einreichen` reicht diese
Parameter nicht durch, deshalb steht dort weiterhin bei 17 Beanstandungen, während die
Funktions-Ratsche (`tests/test_checkest_durchstich.py`) nur noch 3 zählt (Stand nach 6063dda).
Auftrag: `AbsName/AbsStr/AbsPlz/AbsOrt` aus der Deklaration ableiten, `absender_steuernummer`
bleibt Parameter (kein Kz-Spiegel). Kein Code außerhalb `produkt/import/elster_xml.py` +
`tests/test_elster_xml.py` angefasst — `api.py` gehört einem anderen Worker.

## Kz-Herkunft (aus dem E10-2025.xsd, `A_m275349613_CType`, Zeilen ~8490–8580)

| Vorsatz-Feld | Kz | Label | Pflicht für Ableitung |
|---|---|---|---|
| AbsName | E0100201 | Nachname | ja |
| AbsName | E0100301 | Vorname | ja |
| AbsStr | E0101104 | Straße | ja |
| AbsStr | E0101206 | Hausnummer | ja |
| AbsStr | E0101207 | Hausnummerzusatz | optional, wird angehängt |
| AbsPlz | E0100601 | PLZ | ja |
| AbsOrt | E0100602 | Wohnort | ja |
| StNr | — | kein Kz-Spiegel | bleibt Parameter |

Fehlt ein Pflicht-Kz eines Feldes, bleibt das Feld `None` — `erzeuge_xml()` bricht dann
fail-closed ab und nennt in der Meldung das fehlende Kz samt Label, nicht nur den
Parameternamen (`produkt/import/elster_xml.py:_leite_absender_ab`, Aufrufstelle im
`abgabefaehig`-Block).

## Messung 1: reagiert checkESt auf das Format von AbsStr?

`AbsStr` ist laut Schema `String_MinL1_MaxL30_CType` — ein freier String ohne Muster
(E10-2025.xsd:25328). Gemessen, ob checkESt trotzdem eine Formvorgabe durchsetzt (Trennzeichen,
Hausnummer-Pflicht, Zusatz-Notation): 5 Varianten desselben Minimalfalls
(`_dekl(E0100201="Maier")`, `abgabefaehig=True`, restliche Absenderdaten konstant), Skript
`/tmp/.../scratchpad/absstr_format.py`, Lauf gesichert in `absstr_lauf1.txt`:

```
$ set -a && . ./.env && set +a && python3 scratchpad/absstr_format.py | sed "s/$ELSTER_HERSTELLER_ID/<ID>/g"
```

| Variante | AbsStr | rc | n Fehler |
|---|---|---|---|
| Baseline (Referenz-Wert) | `Musterstr. 55` | 610001002 | 6 |
| Ohne Hausnummer | `Musterstr.` | 610001002 | 6 |
| Ohne Leerzeichen | `Musterstr.55` | 610001002 | 6 |
| Mit Zusatz, kein Trenner | `Musterstr. 55a` | 610001002 | 6 |
| Mit Zusatz, mit Trenner | `Musterstr. 55 a` | 610001002 | 6 |

**Befund: identische 6 Fehlermeldungen in allen 5 Varianten** (alle 6 betreffen den
Hauptvordruck — Erklärungsrahmen, Geburtsdatum, Religion, Name, Adresse, Bankverbindung —,
keine einzige nennt AbsStr oder den Vorsatz-Block). checkESt prüft das AbsStr-Format nicht.
Die gewählte Konkatenation (`"{Straße} {Hausnummer}{Zusatz}"`, Leerzeichen vor der Nummer,
Zusatz direkt angehängt) ist damit eine unauffällige, nicht die einzig mögliche Wahl.

## Messung 2: Äquivalenzbeweis Ableitung vs. explizite Parameter

`tests/test_elster_xml.py::test_ableitung_liefert_checkest_dieselbe_fehlerzahl_wie_explizite_parameter`
(`@braucht_eric`) baut zweimal dasselbe XML — einmal mit vollständigen Stammdaten-Kz und NUR
`absender_steuernummer` als Parameter (Rest abgeleitet), einmal mit denselben Kz plus allen
fünf `absender_*`-Parametern explizit (Referenz-Werte) — und vergleicht die checkESt-Fehlerzahl:

```
$ set -a && . ./.env && set +a && python3 -m pytest tests/test_elster_xml.py -q | sed "s/$ELSTER_HERSTELLER_ID/<ID>/g"
35 passed in 3.00s
```

Beide Pfade liefern dieselbe Fehlerzahl — die Ableitung ist für checkESt ununterscheidbar von
expliziten Parametern.

## Steuernummer (Punkt 4 aus dem Auftrag)

Kein Kz-Spiegel und kein Fall-Feld existieren — weder bei HEAD noch im Arbeitsbaum zum
Messzeitpunkt:

```
$ git show HEAD:produkt/bindung/bindung_an_gesamt.yaml | grep -in steuernummer   # 0 Treffer
$ grep -rn "steuernummer" produkt/bindung/*.yaml | grep -vi "absender_steuernummer\|empfaenger"  # 0 Treffer
$ grep -rln "steuernummer" produkt/store/*.py produkt/mapping/*.py               # 0 Treffer
```

`absender_steuernummer` bleibt Parameter, wie im Auftrag festgelegt — kein eigenes Feld
angelegt.

## Mutationsproben (alle rot vor dem Fix, grün danach)

1. Vorrang-Reihenfolge vertauscht (`abgeleitet or explizit` statt `explizit or abgeleitet`) →
   `test_explizit_absender_hat_vorrang_vor_ableitung` rot.
2. Fehlend-Kz-Filter invertiert → `test_teilweise_fehlendes_kz_nennt_nur_das_fehlende` und
   `test_fehlende_ableitung_nennt_das_kz_nicht_nur_den_parameternamen` rot.
3. Hausnummernzusatz-Anhängen entfernt → `test_hausnummernzusatz_wird_an_die_strasse_angehaengt`
   rot.

## Nicht angefasst

`api.py:2398` (`/einreichen`) übergibt weiterhin weder `abgabefaehig=True` noch
`absender_steuernummer` — das bleibt offen, gehört aber laut Auftrag einem anderen Worker.
Mit dieser Ableitung reicht dafür `abgabefaehig=True` + `absender_steuernummer`; die vier
anderen Parameter sind nicht mehr nötig, sofern die Stammdaten-Kz im Fall stehen.

`tests/test_einreichen_durchstich.py::test_delta_endpunkt_funktion_ratsche_bekannt` ist rot,
aber nachweislich unabhängig von dieser Änderung (reproduziert identisch mit
`elster_xml.py`/`test_elster_xml.py` per `git stash` entfernt) — Drift zwischen
Endpunkt-Ratsche (17) und Funktions-Ratsche (3), Ursache liegt in `api.py`/`checkest_gate.py`,
beide aktuell von einem anderen Worker bearbeitet (Arbeitsbaum zeigt `M` auf beiden Dateien).
