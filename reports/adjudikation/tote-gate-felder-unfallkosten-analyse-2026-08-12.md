# Tote Gate-Felder (fam_kinder_*) + Unfallkosten Rz. 30 — Analyse, kein Bau

**Datum:** 2026-08-12
**Auftrag:** Team-lead, reine Untersuchung. Keine Produktions- oder Testdatei geändert. Beide
Themen mit klarer Empfehlung.
**Kurzfassung:** Thema 1 → **LÖSCHEN** (nicht verdrahten). Thema 2 → **NICHT BAUEN** (Streitstand
bleibt dokumentierter Claim).

---

## Thema 1 — Tote Gate-Felder `fam_kinder_im_haushalt` / `fam_kinder_beruecksichtigt`

### Wo sie existieren (gemessen)

Beide Felder stehen als vollständige Bindungs-Einträge in
`produkt/bindung/bindung_kap_vv_familie.yaml:490` (`fam_kinder_im_haushalt`) und `:515`
(`fam_kinder_beruecksichtigt`) — `typ: bool`, `askable: true`, mit `fragetext_laie`, `hilfe_kurz`,
`anker_ref`. Beide tragen `elster_kz: null` mit explizitem `elster_kz_grund` (kein Kz-Mapping
möglich/gewollt).

Befehl: `git show HEAD:produkt/bindung/bindung_kap_vv_familie.yaml` (Zeilen 480–530).

Sie erscheinen sonst nur noch an zwei Stellen im Repo:
- `tests/test_bindungstabelle.py:371` — als Einträge in `UNERREICHBAR_BEKANNT` (dazu unten mehr,
  das ist der wichtigste Fund).
- `BACKLOG.yaml:1362` — Punkt 8 des Zufallsfund-Sweeps vom 2026-08-11, der exakt dieselbe Frage
  schon einmal nachgemessen hat (0 Treffer in api.py/runner.py, Stand unverändert).

Befehl: `grep -rn "fam_kinder_im_haushalt\|fam_kinder_beruecksichtigt" --include="*.py" --include="*.yaml" --include="*.md" .`

### Was sie tun sollten (gemessen, `git show 92cdca69` im Volltext)

Commit 92cdca69 (2026-07-21, "Bug2") hat sie beim Schärfen des Fragetexts von `fam_anzahl_kinder`
als DEFERRED markiert, nicht gelöscht — mit der Begründung: Voll-Verdrahtung als Ring-Gate sei eine
größere Änderung, MVP-Mitigation stattdessen: `fam_anzahl_kinder` fragt jetzt schon explizit nach
der *anspruchsberechtigten* Kinderzahl (Kindergeld/Kinderfreibetrag), nicht mehr nach "Kinder im
Haushalt". Zitat aus dem Commit-Kommentar in der YAML:

> „Feld ist TOT — kein Accessor in golden/runner.py, kein Lesezugriff in produkt/haut/api.py […]
> MVP-Mitigation stattdessen: fam_anzahl_kinder ist per geschärfter Selbstdeklaration bereits die
> anspruchsberechtigte Kinderzahl."

**Regelbezug (gemessen über `pipeline/produktion/rules.yaml:873` und `:1405`, sowie
`golden/runner.py`):**

- `fam_kinder_im_haushalt` gehört zu `p24b_entlastungsbetrag` (§ 24b Entlastungsbetrag
  Alleinerziehende). Diese Regel **ist live** — `golden/runner.py:627` (`catala_p24b_entlastung`)
  ruft das Catala-Modul `EB.entlastungsbetrag()` tatsächlich auf, und `produkt/haut/api.py:984`
  speist es mit genau drei Werten: `alleinstehend`, `anzahl_kinder`, `monate_ohne_voraussetzung`.
  Die kompilierte Catala-Signatur hat **keinen vierten Parameter** für die
  Geltungsbedingung `kinder_mit_freibetrag_oder_kindergeld_im_haushalt` — das Feld kann strukturell
  nie ankommen, nicht "noch nicht verdrahtet", sondern es gibt keinen Slot dafür.
- `fam_kinder_beruecksichtigt` gehört zu `p32_6_kinderfreibetraege`. Diese Regel wird **nirgends**
  aufgerufen (`grep -n "p32_6_kinderfreibetraege" golden/runner.py produkt/haut/api.py` → 0
  Treffer). Der tatsächliche Kinderfreibetrag läuft über eine komplett andere, simple
  Parameter-Lookup-Funktion `runner._kinderfreibetrag(vz, veranlagung)` (`golden/runner.py:1183`),
  multipliziert mit `fam_anzahl_kinder` — kein Catala-Aufruf, kein Gate, keine Prüfung von
  `fam_kinder_beruecksichtigt` oder irgendeines der anderen p32_6-Felder für die *Betrags*rechnung.

Das ist eine wichtige Präzisierung gegenüber dem Commit-Text: bei p24b ist die *Regel* lebendig und
nur das *Gate-Feld* tot; bei p32_6 ist die *ganze Catala-Regel* für die Betragsrechnung tot, nicht
nur ein Feld.

### Werden sie im Dialog gestellt? — Neuer Befund, wichtiger als die reine Accessor-Frage

Das ist NICHT der Fall, und der Grund ist stärker als "kein Accessor liest sie". Es gibt bereits
einen bestehenden, grünen Test, der das seit einiger Zeit dokumentiert und gattert:
`tests/test_bindungstabelle.py:357-393`, Test `test_g_askable_felder_sind_erreichbar`. Die Liste
`UNERREICHBAR_BEKANNT` (Zeile 367-382) führt beide Felder als bekannt-unerreichbar mit Kommentar:

> „Felder, die askable gebunden sind, aber in KEINER nutzerwählbaren Scheibe stehen. Die Oberfläche
> bietet nur 'gesamt' und 'rentner_gesamt' […], also kann der Nutzer sie nicht setzen — ein POST
> endet mit 'feld_id nicht in dieser Scheibe'. Wer so ein Feld verdrahtet, baut ins Leere."

Verifiziert im Code selbst: `produkt/haut/api.py:2342` (`fragen()`) und `:2367` (`stand()`) rufen
`TR.naechste_fragen()`/`TR.relevanz()` NICHT mit der Voll-Bindung auf, sondern mit
`bindung = _scheibe_bindung(store)` (`api.py:122`) — das filtert die Bindung VOR dem Traverser auf
genau die Feld-Liste der aktuellen Scheibe. `api_constants.py:234/240` listet die tatsächlich in
"gesamt" enthaltenen p24b/p32_6-Felder explizit (`fam_alleinstehend`, `fam_anzahl_kinder`,
`fam_monate_ohne_voraussetzung` sind drin) — `fam_kinder_im_haushalt`/`fam_kinder_beruecksichtigt`
fehlen in diesen Tupeln.

**Konsequenz:** In einer echten Session sind die zwei Felder gar nicht erst Kandidat im
Traverser (`naechste_fragen`/`relevanz` sehen sie nicht, weil `bindung` sie nicht enthält). Sie
werden dem Nutzer heute **nicht gestellt** und können auch keine Regel fälschlich ausschließen
(mein erster Verdacht — dass ein "Nein" auf das tote Gate-Feld die echten, live gelesenen Slots
`fam_alleinstehend` etc. aus der Frage-Warteschlange kippen könnte — trägt NICHT, weil das Feld nie
in die Kandidatenliste kommt, um überhaupt beantwortet zu werden).

Sie kosten den Nutzer also **aktuell keine Antwort**. Das ändert die Einschätzung gegenüber der
Auftrags-Prämisse: es ist nicht "totes UND lautlos gefragtes" Feld, sondern doppelt tot — weder
erreichbar noch gelesen. Das ist trotzdem kein Grund, sie liegen zu lassen (siehe Empfehlung).

### Ersatz durch `kind_unter_14_haushaltszugehoerig`? — Nein, andere Regel

`kind_unter_14_haushaltszugehoerig` (Commit 1e140d2, 2026-08-11) gehört zu `p10_1_5_kinderbetreuung`
(§ 10 Abs. 1 Nr. 5, Kinderbetreuungskosten) — eine DRITTE, eigene Regel, verschieden von § 24b
(Entlastungsbetrag) und § 32 Abs. 6 (Kinderfreibetrag). Es ist **live**: `api.py:719` und `:1387`
lesen es direkt (`inst["felder"].get("kind_unter_14_haushaltszugehoerig", {}).get("wert")`) als
Qualifikationsgate pro Kind-Instanz für den Betreuungskosten-Abzug. Konzeptionell verwandt (alle
drei Regeln fragen "zählt dieses Kind"), aber rechtlich unterschiedliche Tatbestände mit
unterschiedlichen Voraussetzungen (Alter <14 bei Betreuung vs. Kindergeld-/Freibetragsanspruch bei
§24b/§32) — kein 1:1-Ersatz, keine Kannibalisierung.

### Redundanz mit `fam_anzahl_kinder` (gemessen, aus 92cdca69 selbst)

Der eigentliche Grund, warum Nachbau sich nicht lohnt: seit 92cdca69 fragt `fam_anzahl_kinder`
bereits explizit nach der Anzahl der Kinder mit Kindergeld-/Kinderfreibetrag-Anspruch — **demselben
Konzept**, das `fam_kinder_im_haushalt` (Ja/Nein) und `fam_kinder_beruecksichtigt` (Ja/Nein) separat
abfragen würden. Eine Voll-Verdrahtung würde dem Nutzer dieselbe Tatsache zweimal abverlangen (einmal
als Zahl, einmal als Ja/Nein) — mit dem Risiko, dass beide Antworten widersprüchlich einlaufen und
niemand das prüft.

### Kz-Kandidaten-Reports beantworten die Frage nicht (gemessen)

Beide vom Team-lead genannten Reports (`2026-07-17-kz-kandidaten-anlage-r-kind.md`,
`2026-07-20-ui-feldkatalog-eventschema-designlock.md`) nennen die Feldnamen nur unter dem
Kz-Mapping- bzw. Askability-Klassifikations-Aspekt:
- Report 1 stuft `fam_kinder_im_haushalt`/`fam_kinder_beruecksichtigt` als "GAP" ein (keine passende
  Elster-Kennziffer, Konzept-Mismatch mit E0500702/E0500807).
- Report 2 stuft sie in der Kategorie "HUMAN-ONLY / Kinder-Zuordnung" ein (rechtliche Zuordnung,
  nicht LLM-/Beleg-ableitbar).

Keiner der beiden sagt etwas zur Accessor-/Erreichbarkeits-Frage. Das deckt sich mit der eigenen
Einschätzung in `BACKLOG.yaml:1362-1369`, die exakt dasselbe schon einmal festgestellt hat ("die
eigentliche Aussage aus 92cdca69 steht in keinem der beiden"). Neu gegenüber dem Backlog-Eintrag ist
hier ausschließlich der `UNERREICHBAR_BEKANNT`-Fund (Nicht-Erreichbarkeit über die Scheibe, nicht nur
fehlender Accessor).

### Empfehlung: LÖSCHEN

Begründung in Kurzform:
1. Gemessen tot auf drei unabhängigen Ebenen: kein Accessor (Ring), keine Kz (XSD-Submission), keine
   Scheibe-Erreichbarkeit (UI) — alle drei Reports/Tests, die diese Achsen unabhängig geprüft haben,
   kommen zum selben Nullbefund.
2. Gemessen redundant: `fam_anzahl_kinder` deckt seit 92cdca69 dasselbe Tatbestandsmerkmal ab.
3. Kein Under-tax-Risiko durch Liegenlassen (nichts wird gefragt, nichts wird stillschweigend
   ignoriert) — aber auch kein Nutzen durch Verdrahten, außer einer doppelten Frage.
4. Aufwand Verdrahten wäre nicht klein: p24b bräuchte eine Catala-Signatur-Erweiterung (neuer
   Parameter im kompilierten Modul) oder eine reine Python-Vorprüfung außerhalb Catala; p32_6
   bräuchte den kompletten fehlenden Catala-Aufruf ODER eine Python-Nachbildung der
   Berücksichtigungsprüfung — beides > 1h, für eine Information, die bereits in `fam_anzahl_kinder`
   steckt.
5. Löschen bedeutet: Bindungs-Einträge entfernen, `UNERREICHBAR_BEKANNT`-Zeile 371 kürzen
   (`kind_idnr` etc. bleiben, nur die zwei Feld-Namen raus), BACKLOG-Punkt 8 als erledigt markieren.
   Geschätzter Aufwand: < 30 min, kein Ring-/Catala-Touch.

**Gemessen / vermutet, sauber getrennt:** Accessor-Nullbefund, Scheibe-Nichterreichbarkeit,
Redundanz mit `fam_anzahl_kinder`, Regel-Aufruf-Status (p24b live/p32_6 tot) — alles **gemessen** via
grep + Code-Lesen wie oben zitiert. Die Einschätzung "Verdrahten würde zu Doppel-Antworten führen"
ist eine **Bewertung**, keine Messung, aber direkt aus dem gemessenen Fragetext von
`fam_anzahl_kinder` abgeleitet.

---

## Thema 2 — Unfallkosten, Rz. 30 (BMF contra BFH VI R 8/18)

### Worum es inhaltlich geht (Quelle im Repo vorhanden, verifiziert)

`sources/bmf/bmf_entfernungspauschalen_2021-11-18.txt:47-53` (BMF-Schreiben vom 18.11.2021,
IV C 5 - S 2351/20/10001 :002, BStBl I 2021, 2315; `authority: verwaltung`, sha256 im
Meta-File hinterlegt) — Wortlaut:

> „Rz. 30: Durch die Entfernungspauschale sind sämtliche Aufwendungen abgegolten. Unfallkosten
> koennen als aussergewoehnliche Aufwendungen (§ 9 Abs. 1 Satz 1 EStG) neben der
> Entfernungspauschale beruecksichtigt werden. Die Finanzverwaltung folgt insoweit nicht der
> Rechtsprechung des BFH (Urteil vom 19.12.2019, VI R 8/18), wonach Unfallkosten nicht als
> Werbungskosten neben der Entfernungspauschale abziehbar seien - AUTHORITY-KONFLIKT verwaltung vs
> bfh."

D.h. konkret:
- **BMF/Verwaltung:** Entfernungspauschale gilt grundsätzlich abgeltend, ABER Unfallkosten (Reparatur-
  /Selbstbehalt-Kosten aus einem Verkehrsunfall auf dem Arbeitsweg) sind trotzdem zusätzlich
  abziehbar — als außergewöhnliche Aufwendungen unter der allgemeinen Werbungskosten-Generalklausel
  § 9 Abs. 1 S. 1 EStG, außerhalb der Pauschale.
- **BFH VI R 8/18 (19.12.2019):** Entfernungspauschale ist vollständig abgeltend, auch für
  Unfallkosten — kein Zusatzabzug.

Die Finanzverwaltung ist hier die **großzügigere** Seite (nicht wie sonst oft umgekehrt): sie hält an
der älteren, steuerpflichtigen-freundlicheren Praxis fest, obwohl der BFH sie zurückgewiesen hat.
Für den Steuerpflichtigen ist das im Ergebnis günstig: er kann sich gegenüber dem Finanzamt auf das
BMF-Schreiben berufen (das Finanzamt ist an seine eigene Verwaltungsanweisung gebunden); erst vor
Gericht (Einspruch/Klage) würde nach aktueller BFH-Linie der Zusatzabzug wackeln.

Ich habe **keine weitere Quelle im Repo** dazu gefunden (der volle BFH-Urteilstext VI R 8/18 selbst
liegt nicht unter `sources/` — nur das BMF-Schreiben, das ihn zitiert). Für die reine
Streitstand-Existenz und die Positionen beider Seiten reicht die obige Fundstelle; für Details des
BFH-Urteils (Tatbestand, genaue Begründung) fehlt die Primärquelle im Repo.

Befehl: `grep -rn -i "unfallkosten" sources/bmf/bmf_entfernungspauschalen_2021-11-18.txt` und
`find sources/ -iname "*VI*R*8*18*"` (kein Treffer für Letzteres, nicht extra ausgeführt oben, aber
`find sources/bmf -iname "*entfernungspauschale*"` zeigt nur die zwei Dateien txt+meta.yaml — kein
separates BFH-Dokument im Repo).

### Wen trifft es — ist Julius' Zielfall betroffen?

Strukturell: JA, in der Personengruppe. Entfernungspauschale ist gebaut und live (gemessen:
`golden/runner.py:40` importiert `Entfernungspauschale as EP`, `produkt/bindung/bindung_n_vor_gwg.yaml`
bindet `ep_entfernung_km`). Unfallkosten sind ein reiner ADD-ON zur Entfernungspauschale — relevant
für jeden Arbeitnehmer, der die Pauschale in Anspruch nimmt. Zusammenveranlagung ändert daran nichts
(Unfallkosten sind personenbezogen, nicht veranlagungsartabhängig).

Faktisch: **nicht feststellbar aus dem Repo.** Der Tatbestand ist ereignisgebunden — er greift nur,
wenn im Steuerjahr tatsächlich ein Verkehrsunfall auf dem Arbeitsweg mit nicht erstatteten Kosten
stattgefunden hat. Dazu liegen keine Daten im Repo vor (weder Golden-Fall noch Store-Eintrag). Das ist
eine Frage an Julius, keine, die sich messen lässt.

### Größenordnung des Geldeffekts

**Vermutet, nicht gemessen** (kein passender Golden-Fall, kein Parameter im Repo): der Effekt ist
NICHT wie bei den meisten anderen Backlog-Punkten ein systematischer, für jeden Nutzer wiederkehrender
Betrag, sondern ein **seltenes Einzelereignis**. Wenn es eintritt, ist die Bandbreite groß
(Bagatellschaden im niedrigen dreistelligen Bereich bis Totalschaden im vierstelligen Bereich,
abzüglich Versicherungsleistung), multipliziert mit dem Grenzsteuersatz. Anders als bei
Entfernungspauschale/Kinderfreibetrag betrifft es nicht die Grundgesamtheit der Nutzer, sondern nur
die (kleine) Teilmenge mit einem Unfall im Veranlagungszeitraum. Eine seriöse Zahl ohne Fallbezug ist
nicht seriös zu nennen — ich nenne bewusst keine.

### Wie würde man es bauen, und was ist der ehrliche Umgang mit einem Streitstand?

**Repo-Präzedenz für "Streitstand" (gemessen):** Es gibt genau EINEN 'streitstand'-Claim im gesamten
Docstore (`docstore/schema.sql:30` definiert den Enum-Wert `'streitstand'` als Kategorie,
`docstore/ingest.py:162-181` erzeugt genau einen INSERT mit `typ='streitstand'` — und das ist exakt
dieser Unfallkosten-Fall). Es gibt also **keine andere Stelle im Repo**, an der ein echter
Autoritäten-Konflikt (Verwaltung vs. Rechtsprechung) bereits zu einer gebauten Regel geführt hätte,
die ich als Bauform-Vorlage zitieren könnte. Das ist ein Negativbefund, kein Ausweichen — ich habe
gezielt nach "Streitstand", "authority-konflikt", "strittig" über `produkt/`, `rules/`, `golden/`,
`BACKLOG.yaml`, `reports/` gesucht.

Was es im Repo an VERWANDTEN, aber nicht identischen Mustern gibt:
- **Günstiger-Vergleich** (`produkt/traverser/guenstiger_liste.yaml`, z. B. § 31
  Familienleistungsausgleich, § 32d Abgeltungsteuer-Günstigerprüfung): der Ring rechnet BEIDE
  Varianten und wählt automatisch die für den Nutzer günstigere. Das löst aber ein anderes Problem
  — dort schreibt das GESETZ SELBST den Vergleich vor (unstrittig, welche Variante wann gilt), hier
  ist STRITTIG, ob der Zusatzabzug überhaupt zulässig ist. Kein 1:1-Vorbild.
- **`kap_zusammenveranlagung` gestrichen** (`bindung_kap_vv_familie.yaml:699`, referenziert im
  Kz-Report oben): Präzedenz dafür, eine zweite Abfrage zu STREICHEN statt sie zu bauen, wenn ein
  bereits vorhandenes Feld (dort: `veranlagung`) denselben Sachverhalt trägt und Widerspruch riskiert
  — dieselbe Logik wie meine Löschen-Empfehlung in Thema 1, aber kein Streitstand-Fall.

**Wenn man es bauen würde:** ein ehrlicher Bau dürfte NICHT stillschweigend einer Seite folgen
(weder BMF-blind noch BFH-blind), sondern müsste den Konflikt im Produkt sichtbar machen — analog zu
den bestehenden Mustern für unsichere/geltungsbedingte Felder in diesem Repo (`annahmen_offen` in
`traverser.relevanz()`, das bewusst NIE eine unbeantwortete Annahme still als erfüllt behandelt,
sondern offen ausweist). Konkret hieße das vermutlich: ein askable Feld "hattest du im VZ einen
Unfall auf dem Arbeitsweg mit nicht erstatteten Kosten?" + Betragsfeld, mit einem expliziten
Hinweistext im UI, der den Streitstand benennt (BMF erlaubt es, BFH hat dagegen entschieden;
Abzug ist möglich, aber im Streitfall vor Gericht angreifbar) — eher ein **Wahlrecht mit Hinweis**
als ein automatischer Abzug. Das ist eine Bauform-Skizze, keine Empfehlung zu bauen (s. u.).

### Empfehlung: NICHT BAUEN

1. Kein gemessener Geldeffekt für den konkreten Zielfall (ereignisgebunden, keine Daten im Repo, ob
   das Ereignis überhaupt vorliegt).
2. Kein Bauform-Präzedenz im Repo, die das Streitstand-Risiko sauber abbilden würde — würde man es
   naiv bauen (einfacher Zusatzabzug ohne Hinweis), würde man stillschweigend der BMF-Seite folgen
   und dem Nutzer einen Abzug geben, der bei einer BFH-konformen Prüfung streitig wäre. Das
   widerspricht der sonst im Repo gelebten Praxis, Streitstände offen zu halten statt sie durch die
   Implementierung climbing zu entscheiden.
3. Der bisherige Umgang (als 'streitstand'-Claim markiert, nicht formalisiert, MVP-Scope-Ausschluss)
   ist konsistent mit dem einzigen Präzedenzfall (es gibt nur diesen einen) und ist der richtige
   Zustand, bis entweder (a) ein konkreter Fall mit tatsächlichem Unfall auftaucht, der den
   Geldeffekt beziffert, oder (b) Julius eine Grundsatzentscheidung trifft, ob das Produkt
   Streitstände als Wahlrecht-mit-Hinweis überhaupt abbilden soll (Scope-Frage, nicht nur
   Unfallkosten-spezifisch).

**Gemessen / vermutet:** BMF-Text + BFH-Aktenzeichen/Datum sind **gemessen** (Primärquelle im Repo,
Zitat oben). Der Geldeffekt ist ausdrücklich **vermutet/nicht quantifizierbar** (keine Daten). Die
Aussage "kein anderer Streitstand ist im Repo gebaut" ist **gemessen** (grep-Negativbefund, s.o.).
Die Bauform-Skizze (Wahlrecht mit Hinweis) ist eine **Bewertung/Vorschlag**, kein gemessener Fakt.

---

## Werkzeuge / Befehle (Nachvollziehbarkeit)

- `git show 92cdca69`, `git show a2a89c06`, `git show HEAD:BACKLOG.yaml`
- `grep -rn "fam_kinder_im_haushalt\|fam_kinder_beruecksichtigt" --include="*.py" --include="*.yaml" --include="*.md" .`
- `grep -n "p24b_entlastungsbetrag\|p32_6_kinderfreibetraege" golden/runner.py produkt/haut/api.py`
- `grep -n "kind_unter_14_haushaltszugehoerig" -r produkt/ tests/`
- `grep -rn -i "unfallkosten" produkt/ rules/ golden/` → 0 Treffer (heute erneut gemessen)
- `grep -rn -i "streitstand\|authority-konflikt\|strittig" --include="*.py" --include="*.yaml" --include="*.md" .`
- Traverser-Pfad gelesen: `produkt/traverser/traverser.py` (`relevanz`, `naechste_fragen`),
  `produkt/haut/api.py:104-129` (`_cfg`, `_scheibe_felder`, `_scheibe_bindung`), `:2338-2360`
  (`fragen()`), `:955-990` (`ent24b`-Berechnung, direkter Store-Read ohne Traverser-Gate).
- `tests/test_bindungstabelle.py:357-393` (`UNERREICHBAR_BEKANNT`, `test_g_askable_felder_sind_erreichbar`)
- Kein Push, kein Force, keine Produktions-/Testdatei geändert. Nur dieser Report geschrieben.
