# Entscheidungsvorlage: die 3 Restfehler (checkESt, `abgabefaehig=True`)

Auftrag team-lead, 2026-08-10. Reines Messen, keine Implementierung — kein Produktionscode
geändert, nichts committet. Messskript im Scratchpad:
`/tmp/claude-1000/-home-julius-00-projects-168-TaxGraph-taxgraph/db772487-ddbf-429b-a0e4-37a31085356e/scratchpad/entscheidungsvorlage_messung.py`.
Hersteller-ID nirgends im Klartext (`<HID>`).

Umgebung:

```
cd /home/julius/00_projects/168_TaxGraph/taxgraph
set -a; . ./.env; set +a
python3 <scratch>/entscheidungsvorlage_messung.py | sed "s/$ELSTER_HERSTELLER_ID/<HID>/g"
```

ERiC 44.2.4.0, Datenart `ESt_2025`. Baseline = `_fall_einzel()` aus
`tests/test_checkest_durchstich.py` (RESTFEHLER_EINZEL=3), unverändert übernommen —
Methodik: Kz-Injektion in `est_mapping.deklariere()`-Output, dann `erzeuge_xml()` +
amtliches `checkESt`. Alle Zahlen unten sind **frisch gemessen in diesem Auftrag**, nicht
aus dem Gedächtnis übernommen.

**TL;DR für die 5-Minuten-Lesung:** alle drei Entscheidungen sind einzeln lösbar und
schließen den `einzel`-Fall auf 0 Restfehler, WENN Julius sich für die "billige" Option
entscheidet (nicht deklarieren / Steuerklasse+Lohnsteuer als 0,00). Die "vollständige"
Option bei Entscheidung 1 (Angabegrund) ist teurer als sie aussieht — keine der drei
Angabegründe schließt sauber, jede zieht mindestens einen neuen Pflichtfragebogen nach
sich (Sparer-Pauschbetrag, KapESt-Detailangaben).

---

## Entscheidung 1 — Nulldeklaration bei Kapitalerträgen

**Frage in einem Satz:** Wenn ein Arbeitnehmer ohne Kapitalerträge die KAP-Kegelfelder
mit 0 bestätigt — soll das XML das als `E19*=0` deklarieren (und dann einen
Angabegrund mitliefern) oder gar nicht erst schreiben?

### Option A — 0-Werte nicht deklarieren

Vier KAP-Kz aus der `deklaration` entfernt (`E1900701` `kap_kapitalertraege`,
`E1900901` `kap_gewinn_aktien`, `E1901201` `kap_verlust_sonstige`,
`E1901301` `kap_verlust_aktien`).

| | vorher | nachher | neue Beanstandungen |
|---|---|---|---|
| einzel | 3 | **2** | keine |

Die KAP-Angabegrund-Meldung verschwindet sauber, die anderen zwei (Steuerklasse,
Lohnsteuer) bleiben unberührt. Deckt sich mit der Vormessung
(`kap_lohnsteuer_messung_2026-08-09.md`, dort 18→17 auf dem älteren Stand).

**Store-Frage (Julius' eigener Punkt: "gibt es den Unterschied unbeantwortet vs.
bestätigt 0?"):** Ja, auf zwei Ebenen unabhängig vom XML-Verhalten.
`est_mapping.deklariere()` schreibt einen Kz nur, wenn im Snapshot ein **bestätigtes**
Event für das Feld liegt (`est_mapping.py:291-294`, fail-closed) — ein nie gestelltes
Feld erzeugt kein Event, ein bestätigtes 0 erzeugt eines. Zusätzlich ist `KAP_FELDER`
in `produkt/haut/api_constants.py:378-399` sowohl in der `"felder"`- als auch der
`"kegel"`-Liste der `"gesamt"`-Scheibe — die KAP-Felder sind heute ein
Pflicht-Kegelfeld, der Ring blockt (`einkunftsart_nicht_ring_faehig`,
`produkt/haut/api.py:2025`) bis sie beantwortet sind. Der Store und der Ring
unterscheiden "unbeantwortet" und "bestätigt 0" also vollständig — **verloren geht der
Unterschied nur im abgegebenen XML selbst**, wenn Option A naiv als Kz-Filter beim
XML-Schreiben implementiert wird (0-Wert raus, unabhängig vom Zustand). Der
Store/Audit-Trail bleibt intakt, nur das eingereichte Dokument zeigt keine Spur mehr,
dass die Frage gestellt und mit 0 beantwortet wurde.

**Einordnung `stille-Null`-Doktrin (BACKLOG.yaml):** Wenn Option A als reiner
XML-Filter gebaut wird (Kz weglassen, Store unverändert), ist das **Klasse C** — der
Filter arbeitet korrekt, aber im abgegebenen Dokument steht kein Signal mehr, dass 0
eine bestätigte Antwort war und keine Lücke. Das ist nicht automatisch schlimm (die
Kegel-Gate-Logik fängt den eigentlich gefährlichen Fall — "Nutzer *hat* Kapitalerträge,
aber Daten fehlen" — bereits vorher ab, siehe `AN_GESAMT_FLAGS`), aber es ist die
gleiche Bauart wie die bereits gefundenen Klasse-C-Fälle: unsichtbar für den, der nur
das XML liest.

### Option B — Angabegrund-Kz mitliefern

Drei Kandidaten aus dem XSD (`KAP_67907_CType`, `Ja1BaseCType_RABE`, Wert `"1"`):
`E1900401` (Günstigerprüfung), `E1900501` (Überprüfung Steuereinbehalt),
`E1900601` (Kirchensteuer-Nacherhebungserklärung) — genau die drei, die checkESt in
seiner Fehlermeldung nennt.

| Angabegrund-Kz | vorher | nachher | neue Beanstandungen |
|---|---|---|---|
| `E1900401` Günstigerprüfung | 3 | 3 | **Sparer-Pauschbetrag fehlt** (ersetzt die alte Meldung 1:1, kein Nettogewinn) |
| `E1900501` Überprüfung Steuereinbehalt | 3 | **4** | KapESt-Detailangaben fehlen + Sparer-Pauschbetrag fehlt (2 neue) |
| `E1900601` KiSt-Nacherhebung | 3 | **4** | KapESt-Detailangaben fehlen + Sparer-Pauschbetrag fehlt (2 neue) |

**Keine der drei Optionen schließt sauber mit nur einem zusätzlichen Kz.** Jeder
Angabegrund zieht mindestens den Sparer-Pauschbetrag als Pflichtangabe nach sich, zwei
davon zusätzlich Detailangaben zur einbehaltenen Kapitalertragsteuer. Option B ist also
kein Ein-Kz-Bau, sondern öffnet einen eigenen kleinen Unterfragebogen — genau die
"Kopplungsfalle", vor der team-lead gewarnt hat.

**Einordnung:** Wird Option B gebaut, ist ein fehlender Sparer-Pauschbetrag (falls
übersehen) **Klasse A** — Pflichtfeld der neu geöffneten KAP-Zweigfrage, würde vor
Absendung fail-closed auffallen (checkESt lehnt ab), kein stiller Verlust.

### Was folgt am Bau

- **Option A:** klein. Ein Filter beim Kz-Schreiben (KAP_FELDER-Kz nur einhängen, wenn
  mindestens einer der vier Werte ≠ 0 ist, oder analog zu `kein_kap`-Gate). Kein neues
  Feld, keine neue Bindung.
- **Option B:** größer als es aussieht. Braucht mindestens eine neue askable Frage
  (Sparer-Pauschbetrag), zwei der drei Varianten zusätzlich eine KapESt-Detailfrage —
  reale Neuerhebung von Nutzerdaten, nicht nur ein Deklarations-Flag.

---

## Entscheidung 2 — Lohnsteuer auf Anlage N (E0200301)

**Frage in einem Satz:** Soll `p36_lohnsteuer` (heute schon importiert, aber laut
`bindung_p36_abschlusszahlung.yaml:22-24` bewusst NICHT deklariert, Grund:
Doppel-Erfassung ggü. der eLStB) doch auf Anlage N erscheinen — als 0 oder mit dem
echten Wert?

### Der Befund zur Quellenlage ("eLStB")

Der Begriff **"eLStB" hat weiterhin 0 Treffer** in `sources/` (erneut geprüft, exakter
String). Aber: der zugrunde liegende Rechtsbegriff — § 41b EStG, "elektronische
Lohnsteuerbescheinigung" — existiert echt und mehrfach in `sources/`, und **§ 10 EStG
etabliert genau das Muster, auf dem die Adjudikation beruht** — an anderer Stelle, aber
mit derselben Logik. Zitat (`sources/gesetze-im-internet/estg_p10_2026-07-11.txt`,
Abs. zu Beitragserstattungen):

> "Satz 1 gilt nicht, soweit diese Daten mit der elektronischen Lohnsteuerbescheinigung
> (§ 41b Absatz 1 Satz 2) […] zu übermitteln sind."

Und ein zweites Mal im selben Paragrafen (Vorsorgeaufwendungen):

> "Satz 1 gilt nicht, soweit diese Daten mit der elektronischen Lohnsteuerbescheinigung
> (§ 41b Absatz 1 Satz 2) zu übermitteln sind."

**Das ist der wichtigste Satz dieses Berichts:** das EStG kennt an anderer Stelle
ausdrücklich das Prinzip "was per eLStB schon übermittelt wird, muss nicht zusätzlich
in der Erklärung stehen". Das ist eine **strukturelle Stütze**, kein direkter
Beleg — die zitierten Stellen betreffen Beitragserstattungen und Vorsorgeaufwendungen
(§ 10 EStG), nicht den Lohnsteuerbetrag selbst auf Anlage N. Keine der vier gefundenen
Fundstellen (auch die beiden anderen, zu § 32b Progressionsvorbehalt und
Grenzgänger-Kennzeichnung "FR"/"M") bestätigt wörtlich, dass **der Lohnsteuerbetrag
selbst** von der Erklärungspflicht befreit ist. Eine vollständige § 41b-EStG-Liste
(welche Positionen die eLStB abschließend enthält) liegt nicht als eigene Quelldatei
vor — nur Fragmente, zitiert aus anderen Paragrafen (Nr. 5, Nr. 8 von § 41b Abs. 1
Satz 2). Die Adjudikation bleibt also **plausibel, aber nicht textlich 1:1 belegt.**

### Gemessene Wirkung

**Konfessionslos (Baseline, `kist_konfession="keine"`):**

| Variante | vorher | nachher | neue Beanstandungen |
|---|---|---|---|
| Steuerklasse + Lohnsteuer=0,00 | 3 | **1** | keine (nur noch KAP übrig) |

Sauberer Nettogewinn von 2, bestätigt die Vormessung frisch auf der aktuellen
3-Fehler-Baseline.

**Kirchensteuerpflichtig (`kist_konfession="evangelisch"`, der eigentliche Zielfall —
die meisten Nutzer sind nicht konfessionslos):**

| Variante | vorher | nachher | neue Beanstandungen |
|---|---|---|---|
| Baseline (nur Religion umgestellt, sonst wie `_fall_einzel`) | 3 | **4** | +1: "Kirchensteuer jedoch nicht [angegeben]" |
| Steuerklasse + Lohnsteuer=0,00, OHNE Kirchensteuer | 4 | 2 | Kirchensteuer-Meldung bleibt (erwartet) |
| Steuerklasse + Lohnsteuer=0,00 **+ Kirchensteuer=0,00** | 4 | **1** | keine (nur noch KAP übrig) |
| Steuerklasse + **echte** Lohnsteuer=12500,00 + Kirchensteuer=1125,00 (9 % von Lohnsteuer, zu Bruttoarbeitslohn 60.000 €) | 4 | **1** | keine (nur noch KAP übrig) |

**Kernbefund:** die echte Deklaration (reale Beträge) schließt **genauso sauber** wie
die 0-Deklaration — checkESt prüft hier kein Plausibilitäts-Verhältnis zwischen
Bruttoarbeitslohn und Lohnsteuer/Kirchensteuer, jedenfalls nicht bei diesen Werten.
Beide Varianten sind am Fehlerbild ununterscheidbar; der Unterschied ist ausschließlich,
ob das Dokument eine korrekte oder eine fiktive Zahl trägt. Kirchensteuerpflicht ist wie
in der Vormessung ein Paar-Zwang: Lohnsteuer allein reicht nicht, Kirchensteuer muss
mit.

### Eine Bau-Falle, die erst bei der echten Deklaration entsteht

`E0200301`/`E0200501` sind laut XSD vom Typ
`DezimalzahlNichtNegOhneFuehrNull_MaxL15_MaxVK12_MinNK2_MaxNK2_CType_RABE` — **kein**
`E60`-Präfix-Kz, aber trotzdem exakt 2 Nachkommastellen im Komma-Format
(`"12500,00"`) verlangt, nicht der rohe Integer. `_cent_nach_kz()`
(`produkt/mapping/est_mapping.py:65`) formatiert nur Kz mit `E60`-Präfix als
Komma-String; alle anderen `typ: cent`-Felder bekommt den rohen Integer-Wert
(`wert // 100`). Würde `p36_lohnsteuer` naiv als `elster_kz: E0200301, typ: cent`
gebunden, ohne diese Formatierungslogik zu erweitern, würde das XML einen bloßen
Integer statt des Komma-Strings schreiben — ungetestet in diesem Auftrag, ob checkESt
das als Schema- oder Plausibilitätsfehler zurückweist, aber es ist mit Sicherheit
**nicht** das erwartete Format. Bei der 0-Deklaration-Variante fällt das nicht auf
(`"0,00"` von Hand injiziert), bei einer echten Bindung schon.

### Was folgt am Bau

- **0-Deklaration (Steuerklasse + Lohnsteuer=0,00 + ggf. Kirchensteuer=0,00):** klein.
  Zwei/drei feste String-Konstanten, kein neues Feld, keine neue Formel — kostet nur die
  `_cent_nach_kz`-Falle NICHT, weil der Wert hart "0,00" ist.
- **Echte Deklaration:** mittel. `p36_lohnsteuer` (und für die Kirchensteuer-Paarung ein
  neues Feld, falls es noch keins gibt — im heutigen Bindungsstand nicht geprüft, wäre
  Teil dieses Bau-Tasks) müsste gebunden werden, UND `_cent_nach_kz()` (oder eine eigene
  Formatierungsregel) müsste um `E0200301`/`E0200501` erweitert werden, sonst produziert
  die Bindung ein falsch formatiertes Feld beim ersten scharfen Fall.
- Beide Varianten sind angewiesen auf die Julius-Entscheidung zu Konfession/Kirchensteuer
  — das Zusatzfeld Kirchensteuer existiert heute nicht als eigener befüllter Wert
  (nur `kist_konfession`), müsste für die echte Deklaration neu erhoben oder aus
  vorhandenen KiSt-Berechnungsfeldern abgeleitet werden.

---

## Entscheidung 3 — Steuerklasse (E0200002)

**Frage in einem Satz:** Soll `E0200002` (Steuerklasse, XSD-Enum `"1"`–`"6"`,
`minOccurs="0"`) einfach deklariert werden?

| Variante | vorher | nachher | neue Beanstandungen |
|---|---|---|---|
| + `E0200002="1"`, konfessionslos | 3 | **2** | keine |

Sauber, kein neuer Fehler, keine Kopplung — bestätigt die Vormessung frisch. Steuerklasse
ist die einzige der drei Entscheidungen, bei der auch die naive Einzelaktion (nur dieses
eine Kz, ohne Lohnsteuer/Kirchensteuer) bereits einen Nettogewinn bringt.

### Aufwandsfrage

**Kleiner Baustein, keine Kopplungen.** Das Feld ist heute ungebunden — es fehlt eine
askable Frage (Steuerklasse 1–6, vermutlich schon irgendwo im Stammdaten-Fragebogen als
Konzept vorhanden, z. B. für die Lohnsteuerberechnung) plus eine 1:1-Bindung
`elster_kz: E0200002`. Kein Zusatzfeld, kein Formatierungsproblem (String-Enum, kein
Cent-Wert), keine Rückwirkung auf andere Kz — die Messung zeigt keinerlei
Fehlerverschiebung außerhalb dieses einen Punkts.

---

## Zusammenveranlagung — derselbe Beweis für den eigentlichen Zielfall

Auftrag team-lead, 2026-08-10 (Nachtrag). Basis: `_fall_zusammen()` aus
`tests/test_checkest_durchstich.py` (RESTFEHLER_ZUSAMMEN=6), Methodik unverändert
(Kz-Injektion, kein Produktionscode). Skript:
`/tmp/claude-1000/-home-julius-00-projects-168-TaxGraph-taxgraph/db772487-ddbf-429b-a0e4-37a31085356e/scratchpad/entscheidungsvorlage_zusammen_messung.py`.
Person-B-Kz sind **dieselben Kz-Nummern** wie Person A (`E0200002`, `E0200301`,
`E0200501`, `E1900701`/`0901`/`1201`/`1301`) — Person B liegt im `person_b`-Bucket von
`est_mapping.deklariere()` und wird von `_einhaengen()` mit einem
Person-Diskriminator als eigene Instanz desselben Kz geschrieben
(`produkt/import/elster_xml.py:511-539`), kein eigenes Ehegatten-Kz.

### Schrittweise Messung, konfessionslos (beide `kist_konfession="keine"`)

| Schritt | vorher | nachher | Kommentar |
|---|---|---|---|
| 0 Baseline (`_fall_zusammen`) | — | 6 | KAP A+B, Steuerklasse A+B, Lohnsteuer A+B (2× je Kategorie) |
| 1 KAP-Kz **nur** A entfernt | 6 | 6 | **kein Nettogewinn** — neue Meldung ersetzt die alte, siehe unten |
| 2 KAP-Kz **auch** B entfernt | 6 | 4 | jetzt sauber −2 (beide KAP-Meldungen weg) |
| 3 + Steuerklasse A=1 | 4 | 3 | |
| 4 + Steuerklasse B=1 | 3 | 2 | |
| 5 + Lohnsteuer A=0,00 | 2 | 1 | |
| 6 + Lohnsteuer B=0,00 | 1 | **0** | **rc=0, klasse=plausibel** |

**Der wichtige Befund liegt in Schritt 1, nicht am Ende.** Wird KAP-Option A nur für
EINE Person angewandt (Person A entfernt, Person B noch mit `kap_*=0` deklariert),
bleibt die Fehlerzahl bei 6 — aber die KAP-Angabegrund-Meldung für A verschwindet
NICHT ersatzlos, sie wird durch eine neue, zusammenveranlagungsspezifische Meldung
ersetzt:

> "Sie haben angegeben, dass Sie Angaben für 'PersonA' machen möchten, haben aber
> außer der Angabe im Feld '$/KAP[1]/Person[1]$' keine weiteren Angaben getätigt."

Das ist dieselbe Bauart wie Entscheidung 1/Option B bei `einzel` (ein Fehler wird
gegen einen anderen getauscht, kein Nettogewinn) — hier aber durch **asymmetrische
Anwendung über zwei Personen**, nicht durch eine falsche Kz-Wahl. Der doppelte
Person-Container der Anlage KAP verträgt offenbar keinen Zwischenzustand, in dem eine
Person vollständig fehlt, während die andere noch einen (wenn auch leeren) KAP-Block
trägt. **Konsequenz für den Bau von Option A bei `zusammen`:** die Kz-Entfernung muss
für beide Personen ATOMAR erfolgen (gleicher Schreibvorgang), nicht Person-für-Person
nacheinander — sonst entsteht ein neuer, vorher nicht existierender Fehlertyp auf dem
Weg dorthin, auch wenn der Endzustand sauber ist.

### Konfessions-Achse

**Beide kirchensteuerpflichtig** (`evangelisch`/`evangelisch`):

| Variante | vorher | nachher | neue Beanstandungen |
|---|---|---|---|
| Baseline (nur Konfession geändert) | 6 | **8** | +2: Kirchensteuer-Meldung je Person (exakt das Doppelte von einzel, keine Überraschung) |
| alle 6 billigen Fixes, OHNE Kirchensteuer | 8 | 2 | beide Kirchensteuer-Meldungen bleiben (erwartet) |
| + Kirchensteuer A=0,00 + Kirchensteuer B=0,00 | 2 | **0** | rc=0, klasse=plausibel |

**Gemischt** (A `evangelisch`, B `keine` — der Fall, bei dem am ehesten eine
Extra-Regel zu erwarten wäre):

| Variante | vorher | nachher | neue Beanstandungen |
|---|---|---|---|
| Baseline (nur A's Konfession geändert) | 6 | **7** | +1: nur A's Kirchensteuer-Meldung, B bleibt unberührt |
| alle 6 billigen Fixes, OHNE Kirchensteuer A | 7 | 1 | nur A's Kirchensteuer-Meldung bleibt |
| + Kirchensteuer **nur** A=0,00 (B bleibt ohne, korrekt) | 1 | **0** | rc=0, klasse=plausibel |

**Kein zusammenveranlagungsspezifischer Kopplungsfehler auf der Konfessions-Achse
gefunden.** Die Kirchensteuerpflicht koppelt strikt PRO PERSON — B's Fehlerbild ist im
gemischten Fall byte-identisch zu B's Fehlerbild im konfessionslosen Fall. Auch die von
team-lead vermutete Steuerklassen-Kombinationsregel für Ehegatten (z. B. dass 1/1 für
Verheiratete unplausibel sein könnte, weil das reale Steuerklassenpaar meist 3/5 oder
4/4 ist) **hat checkESt in keinem der drei Läufe angemeldet** — Steuerklasse "1" für
beide Personen gleichzeitig, bei `veranlagung=zusammen`, erreicht in allen drei
Konfessions-Konstellationen rc=0. Das schließt eine spätere Sachbearbeiter-Prüfung
außerhalb von checkESt nicht aus, aber die amtliche Plausibilitätsprüfung selbst kennt
diese Regel nicht (oder prüft sie nicht auf Feldebene).

### Komma-Dezimal-Falle — betrifft Person B identisch

`E0200301`/`E0200501` sind bei Person B **dieselben Kz-Elemente** wie bei Person A,
nur unter einem anderen Instanz-Diskriminator (`instanz={cp: 1}` in `_einhaengen()`,
`elster_xml.py:538`) — der XSD-Typ
(`DezimalzahlNichtNegOhneFuehrNull_MaxL15_MaxVK12_MinNK2_MaxNK2_CType_RABE`, exakt 2
Nachkommastellen, Komma-Format) ist identisch, weil es dasselbe Schema-Element ist.
Die Falle aus Entscheidung 2 (`_cent_nach_kz()` formatiert nur `E60`-Präfix-Kz als
Komma-String, alles andere als rohen Integer) betrifft also **beide Personen mit
demselben Code-Pfad** — es gibt keine separate Formatierungsfunktion für `person_b`,
die geprüft oder gefixt werden müsste. Wer die echte Deklaration für `zusammen` baut,
muss `_cent_nach_kz()` (oder die Formatierung an der PARTNER_INSTANZ-Einhängestelle,
`est_mapping.py:340`) für **beide** Schreibpfade gleichzeitig korrigieren — betroffene
Kz explizit: `E0200301` (Lohnsteuer, Person A + Person B), `E0200501` (Kirchensteuer,
Person A + Person B). Ein Fix, der nur die Klasse-b/1:1-Deklaration
(`est_mapping.py:350`) anpasst, aber nicht die PARTNER_INSTANZ-Zeile, ließe Person B
falsch formatiert zurück.

### Ergebnis für Julius

`zusammen` erreicht **rc=0** unter denselben drei "billigen" Bedingungen wie `einzel`
(KAP nicht deklarieren, Steuerklasse deklarieren, Lohnsteuer als 0,00 deklarieren),
angewandt auf **beide** Personen — gemessen für alle drei Konfessions-Konstellationen
(beide konfessionslos, beide kirchensteuerpflichtig, gemischt), jeweils inklusive
Kirchensteuer=0,00 wo die Kirchensteuerpflicht-Kopplung greift. Keine
zusammenveranlagungsspezifische Zusatzregel verhindert das — die einzige
Zusammenveranlagung-spezifische Erkenntnis ist eine **Bau-Reihenfolge-Warnung**
(KAP-Kz-Entfernung muss atomar über beide Personen erfolgen), keine neue
Fehlerquelle im Endzustand.

---

## Zusammenfassung für die Entscheidung

| Entscheidung | billige Option | teure Option | Empfehlungsfreie Messung |
|---|---|---|---|
| 1 KAP | A: nicht deklarieren (−1, 0 neu, Klasse-C-Risiko im XML) | B: Angabegrund (0 bis −1 netto, aber +1/+2 neue Pflichtfragen, keine der 3 Varianten schließt clean mit nur einem Kz) | gemessen, keine Wertung |
| 2 Lohnsteuer | 0-Deklaration (−2 bzw. −3 mit KiSt-Paar, klein) | echte Deklaration (gleich sauber, aber Formatierungs-Bau + evtl. neues KiSt-Feld) | gemessen, keine Wertung; eLStB-Rechtslage strukturell gestützt, nicht wörtlich belegt |
| 3 Steuerklasse | (nur eine Option) deklarieren | — | −1, 0 neu, kein Kopplungsrisiko, kleiner Baustein |

Werden alle drei "billig" entschieden (A / 0-Deklaration+KiSt-Paar / Steuerklasse
deklarieren) UND kombiniert (nicht nur isoliert getestet), landet `einzel` amtlich bei
**rc=0, klasse=plausibel, 0 Fehler** — gemessen für den kirchensteuerpflichtigen
Zielfall (KAP-Kz entfernt + Steuerklasse=1 + Lohnsteuer=0,00 + Kirchensteuer=0,00).
Der Fall ist damit tatsächlich abgabefähig, kein Rest. **`zusammen` erreicht dieselbe
rc=0 unter denselben Bedingungen, angewandt auf beide Personen — gemessen für alle drei
Konfessions-Konstellationen** (siehe eigener Abschnitt unten). Einzige
zusammenveranlagungsspezifische Erkenntnis: die KAP-Entfernung (Entscheidung 1,
Option A) muss atomar über beide Personen erfolgen, sonst tauscht ein Zwischenzustand
einen Fehler gegen einen neuen, bisher unbekannten.
