# Blast-Radius: `relevanz()` kann nicht-bool-Gates strukturell nie ausschliessen

**Auftrag:** reine Messung, kein Fix, kein Commit ausser diesem Bericht. Beantwortet die
4 Fragen aus `BACKLOG.yaml` Eintrag `relevanz-nicht-bool-gate`
(eingefuehrt in 6b06f3a, Quelle: `reports/adjudikation/dialog_luecke_abgabe_2026-08-09.md`).

**Umgebung:** HEAD `852cb115310981ed7ea8000ca0576ec3fc3f5711`
("docs(BACKLOG): rc=0 erreicht"), 2026-08-10T01:39:56+02:00. Gemessen live gegen den
Arbeitsbaum, nicht gegen `git show HEAD:`, weil der Traverser die Bindung zur Laufzeit von
Platte laedt — dazu ein Vorbehalt im Abschnitt "Sauberkeit der Messung" unten. Jede Zahl ist
mit dem erzeugenden Python-Snippet reproduzierbar; alle Snippets liefen aus dem Scratch-File
`/tmp/claude-1000/.../scratchpad/blast_radius.py` (kein Repo-File, nicht committet).

## Sauberkeit der Messung

Im geteilten Checkout standen zum Messzeitpunkt fuenf Dateien mit unfertigen Aenderungen
anderer Worker (`git status --short`). Geprueft, ob sie meine Zahlen verfaelschen:

- `produkt/bindung/bindung_an_gesamt.yaml` + `produkt/haut/api_constants.py`: fuegen
  `stammdaten_steuernummer` hinzu (`regel_id: p2_festzusetzung_einzel`, **kein**
  `geltungsbedingung` in der `quelle`). Andere Regel als die hier gemessene
  `p2_festzusetzung_zusammen`, kein Gate, taucht in keinem meiner Sets auf. Einziger Effekt:
  die rohen `naechste_fragen()`-Queue-Laengen (181/176/135) sind um 1 Feld hoeher, als sie
  auf einem sauberen HEAD waeren — irrelevant fuer alle Kernzahlen unten, die auf den p2- und
  BLAST-Teilmengen beruhen, nicht auf der rohen Queue-Laenge.
- `produkt/import/elster_xml.py`, `tests/conftest.py`, `tests/test_elster_xml.py`: reiner
  Deklarations-/Test-Pfad, `relevanz()` hat null Aufrufer dort (bereits in der Vorphase
  gegengeprueft: `grep -rn "relevanz(" produkt/` liefert exakt 3 Treffer, keiner in
  `est_mapping.py` oder `elster_xml.py`). Ohne Wirkung auf diese Messung.

Damit: alle Zahlen unten sind gegen den aktuellen HEAD gueltig, mit der einen genannten
Abweichung (+1 irrelevantes Feld in der rohen Queue-Laenge).

## Baseline (unveraendert seit dem urspruenglichen Fund)

```
Bindungen gesamt: 226
askable + geltungsbedingung gesamt: 74
davon bool: 40
davon nicht-bool: 34
```

Neu gemessen (beim urspruenglichen Fund waren es 24 gate-tragende Regeln bei einer Bindung
mit 225 Eintraegen; aktuell 226 Eintraege / 25 gate-tragende Regeln — die Differenz ist die
oben genannte, harmlose `stammdaten_steuernummer`-Aenderung eines anderen Workers, die selbst
kein Gate ist):

```
Gate-tragende Regeln gesamt: 25
```

**5 von 25 Regeln sind "strukturell gefangen"** (jedes ihrer Gates ist nicht-bool, die Regel
kann also nie ausschliessen, egal was der Nutzer antwortet):

| Regel | Gates | Alle Gate-Texte |
|---|---|---|
| `p10_1_3_3a_kv_pv` | 1 | `versicherungsart_bestimmt` |
| `p19_2_versorgungsfreibetrag` | 3 | `versorgungsbeginn_kohorte`, `art_beamtenrechtlich_oder_nicht`, `altersgrenze_sonstige_alter` |
| `p22_1_leibrente_besteuerungsanteil` | 4 | `besteuerungsanteil_aus_kohortentabelle`, `renten_art_basis_oder_ertragsanteil`, `ertragsanteil_alter_bei_rentenbeginn`, `rentenfreibetrag_fixierung_folgejahr` |
| `p23_veraeusserungsgewinn` | 1 | `veraeusserung_innerhalb_frist` |
| `p2_festzusetzung_zusammen` | 10 | 5× `beide_ehegatten_zusammen_veranlagt`, plus 5 Partner-Renten-Weichen (`partner_renten_art_weiche`, `partner_besteuerungsanteil_aus_kohortentabelle`, `partner_ertragsanteil_alter`, `partner_rentenfreibetrag_fixierung_folgejahr`, `versicherungsart_partner_weiche`) |

`p2_festzusetzung_zusammen` hat mehr Gates als beim urspruenglichen Fund vermerkt (10, nicht
5) — die zusaetzlichen 5 sind Partner-gespiegelte Kohorten-/Renten-Weichen, dieselbe Bauart
wie bei `p22_1`. Sie aendern nichts an der Diagnose (unten), weil ein Fix ohnehin auf
Regel-Ebene ansetzen wuerde, nicht auf einzelne Gates.

**Union aller askable Felder dieser 5 Regeln (nicht nur der Gate-Felder selbst) = 54.** Das
ist der wahre obere Rahmen, groesser als die urspruenglich genannten 34, weil jede gefangene
Regel auch reine Slot-Felder ohne eigenes `geltungsbedingung` mitzieht (z. B.
`stammdaten_nachname_partner` traegt selbst kein Gate, haengt aber an `p2_festzusetzung_zusammen`
und bleibt offen, solange die Regel nicht ausgeschlossen wird).

## Frage 1: Wie gross ist der Schaden wirklich?

Drei Szenarien, jeweils ein vollstaendiger, realistischer Store (Stammdaten, Veranlagung,
Kirchensteuer, Lohn/Rente beantwortet — nicht nur `veranlagung` isoliert gesetzt). Gemessen
mit `TR.naechste_fragen()` (die tatsaechliche Dialog-Queue) und `TR.relevanz()`.

| Szenario | Scheibe | Queue-Laenge | `p2_festzusetzung_zusammen`-Status | p2-Felder offen (von 31) |
|---|---|---|---|---|
| Einzelveranlagung ohne Kinder | `gesamt` | 181 | `unentschieden` (Bug — sollte `ausgeschlossen` sein, `veranlagung="einzel"` ist bestaetigt) | **26** |
| Zusammenveranlagung (Kontrolle) | `gesamt` | 176 | `unentschieden` (korrekt — Regel gilt wirklich) | 21 (legitim) |
| Rentnerfall, einzel | `rentner_gesamt` | 135 | `unentschieden` (Bug) | **28** |

Die Zusammen-Zeile ist die Kontrolle: sie zeigt, dass `p2_festzusetzung_zusammen`-Felder
nicht per se immer im Angebot stehen — hier gehoeren 21 davon wirklich dazu, weil die Regel
tatsaechlich zutrifft. Der Unterschied zu den 26/28 der anderen beiden Szenarien ist also
echtes Ueberangebot, kein Messartefakt.

**Realer Schaden = 26 (Einzel) bzw. 28 (Rentner) faelschlich angebotene Felder**, nicht 34
und nicht 54. Der Grund fuer die Differenz zu den 54: von den 5 gefangenen Regeln hat nur
`p2_festzusetzung_zusammen` ueberhaupt eine Bedingung, die im Normalfall unerfuellt sein
kann (Zusammenveranlagung ja/nein) — die anderen 4 Regeln (`p10_1_3_3a_kv_pv`,
`p19_2_versorgungsfreibetrag`, `p22_1_leibrente_besteuerungsanteil`,
`p23_veraeusserungsgewinn`) sind gar keine echten Eligibility-Gates, siehe naechster
Abschnitt — ihre 44-26=18 (Einzel) bzw. 46-28=18 (Rentner) BLAST-Treffer in der Queue sind
kein durch diesen Bug verursachtes Ueberangebot, sondern Baseline-Verhalten, das auch nach
einem Fix unveraendert bliebe.

### Warum nur 1 von 5 Regeln ein echter Bug ist

Gelesen: die Inline-Kommentare an den Gate-Feldern selbst (nicht die freie
`geltungsbedingung`-Bezeichnung, die ist nur Text).

- `bindung_an_gesamt.yaml` (Versorgungsbezuege § 19 Abs. 2): `versorgung_art`-Kommentar
  "Art-Weiche nach dem Alters-Gate ... Steuert Kz-Verzweigung in est_mapping. Kein
  Betragsfeld." — das ist keine Ob-Bedingung, sondern eine Wie-Weiche innerhalb einer immer
  anwendbaren Regel.
- `bindung_n_vor_gwg.yaml:938-940`: `versicherungsart` mit
  `geltungsbedingung: versicherungsart_bestimmt` — jeder Steuerpflichtige hat eine
  Krankenversicherung (gesetzlich oder privat), die Regel `p10_1_3_3a_kv_pv` gilt immer,
  das Feld bestimmt nur, welcher Rechenpfad greift.
- `bindung_rentner.yaml:48`: `renten_art_basis_oder_ertragsanteil` explizit kommentiert als
  "gb-Weiche aa/bb" — Kz-Verzweigung, kein Eligibility-Check. Die anderen 3 Gates von
  `p22_1_leibrente_besteuerungsanteil` folgen derselben Namensbauart (Kohortentabelle,
  Alter-bei-Beginn, Folgejahr-Fixierung — alles Rechenparameter, keine Ob-Fragen).
- `p23_veraeusserungsgewinn` ist ein Multi-Instanz-Opt-in-Muster
  (`instanz_gruppe: p23_veraeusserung`, `bindung_p23_gesamt.yaml`) — Felder entstehen nur,
  wenn der Nutzer aktiv ein Veraeusserungsereignis anlegt. Die 4 Geltungsbedingungen im
  `luecken:`-Abschnitt sind bereits als dauerhaft nicht-auflosbare
  Nutzer-Selbst-Deklaration markiert, nicht als Traverser-Aufgabe gedacht.

Nur `p2_festzusetzung_zusammen`s 5 Gates mit Text `beide_ehegatten_zusammen_veranlagt` sind
eine echte Ob-Bedingung — und dafuer existiert im Store bereits ein bestaetigtes Signal
(`veranlagung`), das relevanz() heute nicht konsultiert, weil es kein eigenes Bindungsfeld
DIESER Regel ist (relevanz() schaut nur auf Felder, die selbst zur Regel gehoeren).

## Frage 2: Kippt es irgendwo von Ueberangebot zu Blockade?

**Nein.** Geprueft: Schnittmenge der real faelschlich offenen p2-Felder mit `kegel` (der
Feldmenge, die tatsaechlich in die Spannen-/Intervallrechnung eingeht), pro Scheibe:

```
gesamt:         p2-offen ∩ kegel = []
an_gesamt:      p2-offen ∩ kegel = []  (im Rentnerfall via rentner_gesamt gemessen)
rentner_gesamt: p2-offen ∩ kegel = []
```

Alle drei leer — nicht weil der Bug harmlos waere, sondern weil `kegel` Partnerfelder
**bewusst und unabhaengig von diesem Bug** ausschliesst (`_ring_bindung()`-Docstring,
`produkt/haut/api.py:2097-2101`: "Sonst zoegen die (bei einzel ungesetzten) Partner-Felder
als unbounded-ohne-Wert das Intervall auf nicht_fixierbar" — bereits vor diesem Bug so
entschieden, bestaetigt im fruehreren Bericht `kegel_luecke_gesamt_2026-08-07.md`). Der
einzige real fehlerhafte Fall (`p2_festzusetzung_zusammen`) trifft die Kegel-Grenze also gar
nicht.

Die anderen 4 gefangenen Regeln haben zusammen 9 (Scheibe `gesamt`/`an_gesamt`) bzw. 13
(`rentner_gesamt`) Felder innerhalb von `kegel` (`basis_kv`, `basis_pv`, `versicherungsart`,
`vorsorge_*`, bei Rentnern zusaetzlich `rentner_alter_bei_rentenbeginn`,
`rentner_jahresrente`, `rentner_renten_art`, `rentner_renten_beginn_jahr`). Das ist aber kein
Blockade-Risiko, weil diese Regeln — wie oben gezeigt — nie eine Ausschluss-Bedingung hatten:
selbst mit einem funktionierenden relevanz()-Fix wuerden sie exakt gleich im Kegel bleiben.
Es gibt nichts, das "entblockt" werden koennte, weil nie etwas blockierte.

**Fazit Frage 2:** kein Kipppunkt, weder aktuell noch hypothetisch nach einem Fix — der eine
echte Bug ist kegel-isoliert durch eine unabhaengige, aeltere Architekturentscheidung; die
kegel-beruehrenden Felder der anderen 4 Regeln sind nie ausschliessbar gewesen.

## Frage 3: Gibt es die Gegenrichtung (Geldfehler)?

**Nein — mit hoher Sicherheit, architektonisch begruendet.** `relevanz()` hat genau 3
Aufrufer im gesamten Repo (`grep -rn "relevanz(" produkt/`):

1. `traverser.naechste_fragen()` — speist ausschliesslich die Dialog-Frage-Queue.
2. `produkt/haut/api.py::stand()` (Z. 2152): `rel = TR.relevanz(...)` landet nur als
   `"relevanz": rel` in der Antwort, reine Anzeige. Die parallel im selben Endpunkt laufende
   Ring-/Intervallrechnung (`_ring_bindung`, `_bescheid_fn`, `IV.intervall`, Z. 2164-2181)
   ist davon vollstaendig unabhaengig — sie haengt nur an `cfg["kegel"]` und den
   materialisierten `felder`, nicht an `rel`.
3. `produkt/haut/api.py::graph()` (Z. 2444): explizit dokumentiert als "Reine Ableitung, EIN
   Traverser-Aufruf, kein Bescheid, kein Schreibpfad."

`grep -rn "relevanz(" produkt/mapping/` liefert null Treffer — `est_mapping.deklariere()`,
der tatsaechliche ELSTER-Deklarationsbauer, konsultiert `relevanz()` nie.

Die einzigen zwei Wege, wie ein Wert tatsaechlich in eine Summe oder ins XML fliesst
(Ring-Intervall ueber `kegel`+materialisierte Felder, und `deklariere()` ueber die
bestaetigten Store-Events), laufen komplett an `relevanz()` vorbei. Ein zu Unrecht als
`unentschieden`/`relevant` markierter Regel-Status kann also **keinen** Cent-Fehler
verursachen — er kann nur eine Frage zu viel stellen (Frage 1) oder eine Anzeige verwirren
(Frage 2, bereits als folgenlos gezeigt). Kein Stop-and-report-Fall gemaess Auftrag, weil
das Ergebnis negativ (sicher) ist, nicht positiv (Risiko) — die Messung wurde daher planmaess
fortgesetzt statt abgebrochen.

## Frage 4: Wie saehe ein Fix aus (Skizze, nicht gebaut)?

**Kernproblem der heutigen Mechanik:** `relevanz()` iteriert nur Bindungseintraege, die
*selbst* zur Regel `rid` gehoeren, und prueft nur, ob *dieses eine Feld* bestaetigt `False`
ist. Es kann nie ein *anderes* Feld (wie `veranlagung`, das zu einer anderen Regel gehoert)
als Ausschlussgrund heranziehen — selbst wenn ein Mensch die Bedingung klar lesen kann.

Ein Fix braucht zwei Teile:

**a) Schema-Erweiterung.** `produkt/bindung/schema.json` muesste neben der heutigen freien
`geltungsbedingung`-Textbezeichnung eine strukturierte, maschinell auswertbare Form erlauben,
die auf ein beliebiges anderes Feld + Wert zeigt, z. B. sinngemaess
`geltungsbedingung: {feld: veranlagung, wert_ungleich: zusammen}` statt nur
`geltungsbedingung: beide_ehegatten_zusammen_veranlagt`. Betroffen: nur die YAML-Stellen mit
echtem Ob-Charakter — laut dieser Messung heute genau die 5 `beide_ehegatten_zusammen_veranlagt`-
Eintraege in `bindung_an_gesamt.yaml`, die andere geltungsbedingung-Texte (branch-selector)
bleiben unveraendert als reine Dokumentation/Annahme.

**b) `traverser.py::relevanz()` umbauen**, um den strukturierten Fall aufzuloesen: statt nur
`bindung.items()` gefiltert auf `q["regel_id"] == rid` zu durchlaufen, muesste die Funktion
bei einer strukturierten Bedingung `aktiv.get(cond["feld"])` (ein Feld ausserhalb der
eigenen Regel) nachschlagen und dessen bestaetigten Wert gegen die Bedingung pruefen — einmal
pro Regel, nicht pro Gate-Feld (alle 10 Gates von `p2_festzusetzung_zusammen` haetten dieselbe
Regel-Bedingung, kein Grund, sie einzeln zu duplizieren).

**Reichweite, wenn nur (a)+(b) gebaut wird:** löst **ausschliesslich**
`p2_festzusetzung_zusammen` (26/28 Felder aus Frage 1), weil das die einzige der 5
gefangenen Regeln mit einem bereits existierenden, bestaetigten Signal (`veranlagung`) ist,
an das sich eine Bedingung haengen liesse. Die anderen 4 Regeln
(`p10_1_3_3a_kv_pv`, `p19_2_versorgungsfreibetrag`, `p22_1_leibrente_besteuerungsanteil`,
`p23_veraeusserungsgewinn`) haben **kein** existierendes Feld, das "gilt diese Regel
ueberhaupt" bedeutet — ihre Gates sind reine Rechen-Weichen. Ein Fix dort waere kein
relevanz()-Codefix mehr, sondern eine Produktentscheidung: ein neues askable-Bool-Feld in der
Bindung anlegen ("Haben Sie ueberhaupt eine private/gesetzliche KV-Pflichtversicherung?" o.ae.),
das es heute nicht gibt — ausserhalb des Scopes dieser Messung.

**Risiko-Check (nicht gebaut, nur geprueft):** 4 Testdateien rufen `relevanz()`/
`naechste_fragen()` (`tests/test_traverser.py`, `tests/test_vpf_frist_unterbrechung.py`,
`tests/test_paket_a_e2e.py`, `tests/test_ui_zwei_signal_sicherheit.py`). Die einzige harte
Status-Assertion (`test_ui_zwei_signal_sicherheit.py:184,191`) prueft `kein_kap` /
`p2_einkunftsarten` — ein bereits funktionierendes Bool-Gate, keine der 5 hier gefangenen
Regeln. Ein additiver Fix (neuer strukturierter Bedingungstyp, bestehende Bool-Logik
unangetastet) wuerde diesen Test nicht beruehren.

**Aufwandseinschaetzung:** klein und lokalisierbar fuer den einen echten Bug (2 Dateien:
`schema.json` + `traverser.py`, plus 5 YAML-Zeilen in `bindung_an_gesamt.yaml`), aber die
Nutzenfrage bleibt offen — der reale Schaden ist ein Dialog-Ueberangebot (26-28 Partnerfelder
in Einzelveranlagung/Rentnerfall), kein Geldfehler und keine Blockade. Julius-Entscheidung.

## Zusammenfassung fuer die Akte

| Frage | Antwort |
|---|---|
| 1. Realer Schaden | 26 (Einzel) / 28 (Rentner) faelschlich angebotene Felder — nicht 34, nicht 54; nur 1 von 5 gefangenen Regeln ist ein echter Bug |
| 2. Kippt zu Blockade? | Nein — 0/0/0 Kegel-Schnittmenge fuer den echten Bug, kegel schliesst Partnerfelder unabhaengig aus; die anderen 4 Regeln waren nie ausschliessbar |
| 3. Geldfehler-Gegenrichtung? | Nein — `relevanz()` hat 3 Aufrufer, alle Dialog-Queue oder reine Anzeige, nie Ring-Rechnung oder `deklariere()` |
| 4. Fix-Skizze | Strukturierte `geltungsbedingung` (Feld+Wert) in `schema.json` + Aufloesung in `relevanz()`; loest nur `p2_festzusetzung_zusammen`, die anderen 4 brauchen ein neues Bindungsfeld (Produktentscheidung) |
