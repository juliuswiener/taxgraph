# Stammdaten-Inventur — 57 fehlende Tags (Produkt-XML vs. Referenz-XML)

Reine Ist-Inventur, keine Implementierung. Primärquelle für Kz-Bedeutung: `kz_extract.py`
gegen `~/02_Software/eric/schema_extract/.../E10-2025.html` (Schema-Pfad geprüft, siehe unten).
Primärquelle für Container-Verschachtelung: `elster/submission/testfall_est2025_minimal.xml`
(vollständig gelesen, 127 Zeilen). Primärquelle für Bindungsstatus: `grep -r` über
`produkt/bindung/*.yaml`, `produkt/mapping/est_mapping.py`, `produkt/store/`.

Schema-Pfad (geprüft via `kz_extract._find_schema('e10')`):
`~/02_Software/eric/schema_extract/ERiC-44.2.4.0/Dokumentation/Datenarten/ElsterErklaerung/ESt/SchemaDokumentation/E10-2025.html`

## Zusammenfassung

| Klasse | Anzahl (von 57) |
|---|---|
| VORHANDEN (gebunden) | 2 |
| FEHLT_STAMMDATUM | 26 |
| FEHLT_RAHMEN | 6 |
| FEHLT_RECHENWERT | 0 |
| UNKLAR / nuanciert | 4 |
| Container (kein eigener Wert, siehe Abschnitt D) | 19 |

19 Container + 38 Werte = 57. Summe Werte-Klassen: 2+26+6+0+4 = 38. ✓

**Wichtigster Befund für den Einhänge-Pfad (Punkt 4):** Die 11 „Container"-Tags (Allg, A, B,
EP, Wk, Erste_Taetig, KiSt_Pfl, AN_Sp_Zul, BV, ESt1A, St_Abz_Betr_Inl_u_Inv_Ert) sind in
`erzeuge_xml()` **kein eigenständiges Problem**: `_einhaengen()` (produkt/import/elster_xml.py:163)
legt jeden Zwischenknoten eines Kz-Pfades automatisch an, sobald irgendein Kz *darunter* deklariert
wird. Sobald z.B. `E0100201` in `deklaration` steht, entstehen `ESt1A/Allg/A` von selbst — keine
Handtabelle nötig. Die Container fehlen also nur, *weil* die 26 Stammdaten-Kz fehlen, nicht
zusätzlich dazu.

**Zweiter wichtiger Befund:** `Vorsatz` und seine 8 Kinder (Unterfallart, Vorgang, StNr, Zeitraum,
AbsName, AbsStr, AbsPlz, AbsOrt, Copyright, OrdNrArt, Rueckuebermittlung/Bescheid) sind **keine
Kz** — keins trägt ein `E\d{7}`-Namensmuster. `kz_pfade()` (elster_xml.py:56) indiziert nur Kz
(`XV.walk()` matcht die Kz-Regex); diese Felder tauchen dort nie auf, egal was in `deklaration`
steht. Der bestehende Mechanismus (Kz-Deklaration → Schema-Walk → Pfad) kann diesen Block
strukturell **nicht** erzeugen — er braucht einen separaten Schreibpfad in `erzeuge_xml()`,
analog zu `_transfer_header()` (Zeile 241), nicht eine Erweiterung von `pflicht_kinder()`.

---

## A. Identität/Stammdaten (26 Kz)

| Kz | Sektion (Schema) | Label (Primärquelle) | Klasse | Beleg / Feld-ID-Vorschlag |
|---|---|---|---|---|
| AbsName | Vorsatz (kein Kz) | — (Freitext, Referenz-Wert `"Maier Hans"`) | FEHLT_STAMMDATUM | Kein Kz-Label (kz_extract findet nur `E\d{7}`); Wert im Referenz-XML ist `Vorname+" "+Name` aus A-Block zusammengesetzt. 0 Treffer für `AbsName` in `produkt/`. Vorschlag: `stammdaten_name_a` (kombiniert aus Vorname/Nachname A) |
| AbsOrt | Vorsatz (kein Kz) | — (Referenz-Wert `"Musterort"`, identisch zu E0100602) | FEHLT_STAMMDATUM | 0 Treffer für `AbsOrt`. Spiegelt E0100602 (Wohnort A) — kein eigenes neues Feld nötig, wenn A-Wohnort existiert |
| AbsPlz | Vorsatz (kein Kz) | — (Referenz-Wert `"55555"`, identisch zu E0100601) | FEHLT_STAMMDATUM | 0 Treffer. Spiegelt E0100601 |
| AbsStr | Vorsatz (kein Kz) | — (Referenz-Wert `"Musterstr. 55"`, **ein** Freitextfeld — anders als E0101104/E0101206 unten getrennt in Straße+Hausnr!) | FEHLT_STAMMDATUM | 0 Treffer. Kombiniert Straße+Hausnummer als ein String; A-Block will sie getrennt (siehe E0101104/E0101206) — beim Schreiben zusammenfügen, nicht zwei Quellen |
| StNr | Vorsatz (kein Kz) | — (Referenz-Wert `"9181081508155"`, Steuernummer) | FEHLT_STAMMDATUM | 0 Treffer für `StNr` in `produkt/`. Vorschlag: `stammdaten_steuernummer` |
| E0100001 | Art_Erkl | „Einkommensteuererklärung" | FEHLT_RAHMEN | Referenz-XML: `X` (identisch im 2020er UND 2025er Testfall). Für dieses Produkt (nur ESt-Erklärungen) konstant `X` — kein Nutzerinput. 0 echte Bindung; einzige Fundstelle `produkt/import/elster_xml.py:169` ist nur ein Docstring-Beispielpfad |
| E0100002 | Art_Erkl | „Festsetzung der Arbeitnehmer-Sparzulage" | UNKLAR (bedingt) | Referenz-XML: `X`. Hängt inhaltlich an E0109109 (AN_Sp_Zul-Container befüllt?) — kein reiner Rahmen-Konstant, aber auch kein Ring-Wert: eher ein *abgeleitetes* Flag „X wenn AN_Sp_Zul-Container Inhalt hat". 0 Bindung gefunden |
| E0100003 | Art_Erkl | „Erklärung zur Feststellung des verbleibenden Verlustvortrags" | UNKLAR (bedingt) | Referenz-XML: `X`. Gehört zu § 10d-Verlustvortragsfeststellung — abhängig davon, ob der Fall das beantragt. 0 Bindung gefunden. Memory-Notiz `[10d EStG (Verlustabzug)]` existiert, aber kein Kz-Bezug dort geprüft (außerhalb Auftrag) |
| E0100201 | A | „Name" | FEHLT_STAMMDATUM | 0 Treffer außer Test-Fixtures (`elster/eric_gate.py:67`, `elster/submission/validate_xsd.py:72` — beides Negativ-Mutationstests auf dem Referenz-XML, keine Produkt-Bindung). Vorschlag: `stammdaten_nachname` |
| E0100301 | A | „Vorname" | FEHLT_STAMMDATUM | 0 Treffer. Vorschlag: `stammdaten_vorname` |
| E0100401 | A | „Geburtsdatum" | FEHLT_STAMMDATUM | 0 Treffer außer Test-Fixture `elster/checkest_gate.py:157` (Negativ-Mutation, keine Bindung). Vorschlag: `stammdaten_geburtsdatum` |
| E0100402 | A | „Religion am 31.12.$VZ$" | FEHLT_STAMMDATUM (verwandtes Feld existiert, unverbunden) | 0 Treffer für E0100402 selbst. **Aber**: `kist_konfession` existiert bereits als askable Feld (`produkt/bindung/bindung_p51a_kirchensteuer.yaml:11`, `signatur_slot: konfession`), explizit **nicht** an ein Kz gebunden — Zeile 22: *„Kein XSD-verifiziertes Kz-Mapping für das Religionsschlüssel-Feld (ESt1A) vorhanden; fail-closed statt Rate-Kz. XSD-Sweep offen (Backlog)."* Vermutlich derselbe Sachverhalt, aber Wertebereich (Laien-Enum vs. amtlicher Religionsschlüssel) ungeprüft — braucht Abgleich, kein neues Feld |
| E0100601 | A | „Postleitzahl (Inland)" | FEHLT_STAMMDATUM | 0 Treffer. Vorschlag: `stammdaten_plz` |
| E0100602 | A | „Wohnort" | FEHLT_STAMMDATUM | 0 Treffer. Vorschlag: `stammdaten_ort` |
| E0100701 | A | „Verheiratet / Lebenspartnerschaft begründet seit dem" | FEHLT_STAMMDATUM | 0 Treffer für ein Datumsfeld. Verwandtes, aber verschiedenes Konzept existiert: ein `veranlagung`-Signal (Werte `"einzeln"`/`"zusammen"`, benutzt in `produkt/haut/api.py:431,1024,1454`) steuert die Splitting-Wahl — das ist ein Ja/Nein-Charakter, **kein** Datum. E0100701 verlangt das genaue Datum, das existiert nirgends |
| E0100801 | B | „Vorname" | FEHLT_STAMMDATUM | 0 Treffer. Vorschlag: `stammdaten_vorname_partner` |
| E0100901 | B | „Name" | FEHLT_STAMMDATUM | 0 Treffer. Vorschlag: `stammdaten_nachname_partner` |
| E0101001 | B | „Geburtsdatum" | FEHLT_STAMMDATUM | 0 Treffer. Vorschlag: `stammdaten_geburtsdatum_partner` |
| E0101002 | B | „Religion am 31.12.$VZ$" | FEHLT_STAMMDATUM | 0 Treffer für `kist_konfession_partner` oder Äquivalent — anders als bei E0100402 existiert für Person B **kein** verwandtes Feld überhaupt (0 Treffer für `feld_id:.*konfession` außer der einen A-Zeile) |
| E0101003 | B | „Ausgeübter Beruf" | FEHLT_STAMMDATUM | 0 Treffer. Vorschlag: `stammdaten_beruf_partner` |
| E0101004 | A | „Titel, akademischer Grad" | FEHLT_STAMMDATUM | 0 Treffer. Optionales Feld (Referenz-Wert `"Dr."`). Vorschlag: `stammdaten_titel` |
| E0101104 | A | „Straße (derzeitige Adresse)" | FEHLT_STAMMDATUM | 0 Treffer. Getrennt von Hausnummer (E0101206) — anders als AbsStr oben. Vorschlag: `stammdaten_strasse` |
| E0101206 | A | „Hausnummer" | FEHLT_STAMMDATUM | 0 Treffer. Vorschlag: `stammdaten_hausnummer` |
| E0101207 | A | „Hausnummerzusatz" | FEHLT_STAMMDATUM | 0 Treffer. Optional (Referenz-Wert `"c"`). Vorschlag: `stammdaten_hausnummerzusatz` |
| E0102002 | BV | „Es ist keine Bankverbindung vorhanden" | FEHLT_STAMMDATUM | 0 Treffer außer Fuzz-Test (`elster/fuzz/checkest_fuzz.py`, keine Bindung). IBAN selbst kommt im ganzen Produkt **nirgends** als Sachverhalts-Feld vor — die einzigen IBAN-Treffer (`produkt/haut/pii_filter.py`, `produkt/import/kontoauszug_writer.py:55,60,63`) sind PII-Maskierung für LLM-Input, keine Stammdaten-Erfassung. Zwei Optionen für die Instructor-Entscheidung: (a) IBAN-Erfassung neu bauen, oder (b) bis dahin `E0102002=X` konstant setzen (Vereinfachung: Erstattung/Nachzahlung läuft dann nicht per Überweisung) |
| E0109109 | AN_Sp_Zul | „Für alle ... übermittelten ... Vermögensbildungsbescheinigungen wird die Festsetzung der Arbeitnehmer-Sparzulage beantragt." | FEHLT_STAMMDATUM | 0 Treffer. Ja/Nein-Ask, hängt mit E0100002 zusammen (siehe oben) |

## B. Erklärungsrahmen (8 Tags — überwiegend keine Kz)

| Tag | Ist ein Kz? | Klasse | Referenz-Wert (beide Testdateien identisch, außer Zeitraum) |
|---|---|---|---|
| Art_Erkl | Nein (Container) | Container, siehe Abschnitt D | — |
| Unterfallart | Nein | FEHLT_RAHMEN | `10` — identisch in `elster/testdaten/est_2020_amtliches_beispiel.xml:134` UND `elster/submission/testfall_est2025_minimal.xml:109` → konstant für Datenart/Vorgang |
| Vorsatz | Nein (Container) | Container, siehe Abschnitt D | — |
| Zeitraum | Nein | FEHLT_RAHMEN, trivial ableitbar | `2025` bzw. `2020` — entspricht exakt dem bereits vorhandenen `vz`-Parameter von `erzeuge_xml()` (elster_xml.py:260). Nur nicht geschrieben, kein neuer Wert nötig: `str(vz)` |
| OrdNrArt | Nein | FEHLT_RAHMEN | `S` — identisch in beiden Testdateien |
| Bescheid | Nein (Kind von Rueckuebermittlung) | FEHLT_RAHMEN | `2` — identisch in beiden Testdateien |
| Rueckuebermittlung | Nein (Container um Bescheid) | Container, siehe Abschnitt D | — |
| Copyright | Nein | FEHLT_RAHMEN | `ELSTER` — identisch (Literal-Signatur der Datenart) |

Nicht in der 57er-Liste, aber Geschwister im selben Vorsatz-Block: `Vorgang` (Wert `01`) — bereits
in `_transfer_header()` (elster_xml.py:246) für den TransferHeader geschrieben, aber das ist ein
**anderes** `<Vorgang>`-Element (anderer Namespace/Kontext: TransferHeader vs. Nutzdaten/Vorsatz).
Nur zur Vollständigkeit erwähnt, nicht Teil der 57 fehlenden Tags.

## C. Werte (12 Kz)

| Kz | Sektion | Label | Klasse | Beleg |
|---|---|---|---|---|
| E0200002 | LStB_1_5_Sum | „Steuerklasse" | FEHLT_STAMMDATUM | 0 Treffer. Nutzer muss sie von der Lohnsteuerbescheinigung ablesen |
| E0200301 | LStB_1_5_Sum | „Lohnsteuer" | UNKLAR — siehe Adjudikation unten | 0 Treffer als direkte Bindung |
| E0200401 | LStB_1_5_Sum | „Solidaritätszuschlag" | UNKLAR — siehe Adjudikation unten | 0 Treffer |
| E0200501 | LStB_1_5_Sum | „Kirchensteuer des Arbeitnehmers" | UNKLAR — siehe Adjudikation unten | 0 Treffer |
| E0203003 | Erste_Taetig | „Ziel des Weges" | FEHLT_STAMMDATUM | 0 Treffer. Sibling von zwei bereits gebundenen Feldern (siehe VORHANDEN unten), selbst aber unverbunden |
| E0203501 | Erste_Taetig | „PLZ, Ort und Straße" | FEHLT_STAMMDATUM | 0 Treffer. Selbe Situation wie E0203003 |
| **E0203503** | Erste_Taetig | „aufgesucht an Tagen" | **VORHANDEN** | `feld_id: ep_arbeitstage`, `produkt/bindung/bindung_n_vor_gwg.yaml:9-20` (`elster_kz: E0203503`, Zeile 20) |
| **E0203504** | Erste_Taetig | „einfache Entfernung in Kilometern" | **VORHANDEN** | `feld_id: ep_entfernung_km`, `produkt/bindung/bindung_n_vor_gwg.yaml:26-37` (`elster_kz: E0203504`, Zeile 37) |
| E1900601 | KiSt_Pfl | „Ich bin kirchensteuerpflichtig und habe Kapitalerträge erzielt, von denen Kapitalertragsteuer, aber keine Kirchensteuer einbehalten wurde." | FEHLT_STAMMDATUM | 0 Treffer in `produkt/`. KAP-Container ist sonst gut gebunden (E1900701, E1900901, E1901301, E1901201 in `bindung_kap_vv_familie.yaml`), aber dieses spezielle Ja/Nein-Flag fehlt |
| E1904701 | St_Abz_Betr_Inl_u_Inv_Ert | „Kapitalertragsteuer" | FEHLT_STAMMDATUM | 0 Treffer für `kapitalertragsteuer_`/`kest`/`kapst`/Äquivalent. Anders als bei Lohnsteuer (siehe unten) keine analoge Adjudikations-Notiz gefunden — Bankdaten werden (anders als eLStB) nicht automatisch elektronisch ans FA übermittelt, ein Doppel-Deklarations-Einwand wie bei E0200301 greift hier vermutlich nicht, aber das ist eine Bewertung, keine Messung |
| E1904801 | St_Abz_Betr_Inl_u_Inv_Ert | „Kirchensteuer zur Kapitalertragsteuer" | FEHLT_STAMMDATUM | 0 Treffer |
| E1904901 | St_Abz_Betr_Inl_u_Inv_Ert | „Solidaritätszuschlag" (zur KapSt) | FEHLT_STAMMDATUM | 0 Treffer |

**VORHANDEN, aber Vorbehalt:** `ep_arbeitstage`/`ep_entfernung_km` sind gebunden — dass sie
trotzdem in den 57 fehlenden Tags auftauchen, heißt vermutlich nicht, dass die Bindung kaputt ist,
sondern dass der Testlauf, der die Tag-Diff erzeugt hat, keine Entfernungspauschale-Werte enthielt.
Beleg: die beiden einzigen im Repo sichtbaren Mess-Skripte für diesen Bereich
(`scripts/measure_veranlagung.py`, `scripts/measure_veranlagung_e2e.py`, beide `git status`
untracked) setzen ausschließlich `bruttoarbeitslohn_*` — 0 Treffer für `ep_arbeitstage`,
`ep_entfernung_km` oder `E0203503`/`E0203504` in beiden Dateien. Das ist eine Lücke im
**Testfall**, keine Lücke im **Mapping** — sollte vor jeder Änderung an `erzeuge_xml()` an diesem
Punkt geprüft werden, sonst wird ein bereits funktionierendes Mapping fälschlich "repariert".

### Adjudikation E0200301/E0200401/E0200501 (Lohnsteuer/Soli/KiSt-AN)

Nicht einfach "fehlt", sondern bereits einmal bewusst **nicht** gemappt — dokumentiert in
`produkt/bindung/bindung_p36_abschlusszahlung.yaml:22-24` (Datum der Entscheidung: 2026-08-05):

> „Die vier Kandidaten mit 'Lohnsteuer' im xs:documentation (E0200301–E0200304) liegen unter
> E10/N/ArbL/LStB_… = Anlage N Zeile 6, also Einkunftsermittlung — ein Mapping dorthin schriebe
> den Betrag doppelt zur eLStB."

Das existierende Feld `p36_lohnsteuer` (gleiche Bindungsdatei, Zeile 13) fragt denselben Betrag
für die § 36-Anrechnung ab, hat aber `elster_kz: null` mit `kz_status: endgueltig` — bewusst kein
E10-Kz, weil der Arbeitgeber die LStB elektronisch ans FA überträgt (eLStB) und eine zusätzliche
Deklaration in der ESt-Erklärung den Betrag doppelt einreichen würde. Ob dieselbe Begründung auch
für E0200301/401/501 gilt (die WOLLEN den Betrag in der Erklärung selbst, nicht nur zur
Anrechnung) oder ob das ein anderer Sachverhalt ist, konnte ich nicht abschließend klären — daher
UNKLAR statt FEHLT_STAMMDATUM. Das ist eine Bewertungsfrage für Julius/Instructor, keine
Recherchelücke: Ich habe geprüft, was es gibt (`p36_lohnsteuer`, `kz_status: endgueltig`,
elster_kz_grund-Text oben), aber ob die 2026-08-05-Entscheidung auf E0200301/401/501 übertragbar
ist, ist eine Norm-/Schema-Frage, keine Grep-Frage.

## D. Container-Verschachtelung (aus `elster/submission/testfall_est2025_minimal.xml`, vollständig gelesen)

```
E10
├── ESt1A                                          (Zeile 27)
│   ├── Art_Erkl                                   (28) → E0100001, E0100002, E0100003
│   ├── Allg                                       (33)
│   │   ├── A                                      (34) → E0100401, E0100201, E0100301,
│   │   │                                              E0101004, E0100402, E0101104,
│   │   │                                              E0101206, E0101207, E0100601,
│   │   │                                              E0100602, E0100701
│   │   ├── B                                      (47) → E0101001, E0100901, E0100801,
│   │   │                                              E0101002, E0101003
│   │   └── BV                                     (54) → E0102002
│   └── AN_Sp_Zul                                  (58) → Person(=PersonA), E0109109
├── N                                              (63) → Person(=PersonA)
│   ├── ArbL                                       (65)
│   │   └── LStB_1_5_Sum                           (66) → E0200002, E0200201(bereits
│   │                                                  gebunden), E0200301, E0200401,
│   │                                                  E0200501
│   └── Wk                                         (74)
│       └── EP                                     (75)
│           └── Erste_Taetig                       (76) → E0203003, E0203501,
│                                                      E0203503(VORHANDEN), E0203504(VORHANDEN)
├── KAP                                            (90) → Person(=PersonA)
│   ├── KiSt_Pfl                                   (92) → E1900601
│   └── St_Abz_Betr_Inl_u_Inv_Ert                  (95) → E1904701, E1904901, E1904801
├── VOR                                            (101)
│   └── AVor                                       (102) → Person, E2000401, E2000801
│                                                      (beide bereits vorhanden, nicht in
│                                                       der 57er-Liste)
└── Vorsatz                                        (108, LETZTES Kind von E10)
    ├── Unterfallart, Vorgang, StNr, Zeitraum       (109-112)
    ├── AbsName, AbsStr, AbsPlz, AbsOrt             (113-116)
    ├── Copyright, OrdNrArt                         (117-118)
    └── Rueckuebermittlung                          (119)
        └── Bescheid                                (120)
```

Reihenfolge-Hinweis für den Einhänge-Pfad: `ESt1A`, `N`, `KAP`, `VOR`, `Vorsatz` sind Geschwister
direkt unter `E10`, in genau dieser Reihenfolge — `erzeuge_xml()` läuft bereits äußere-Schleife-
über-Kz-in-Schema-Reihenfolge (Kommentar Zeile 274-275: „N(PersonA) und N(PersonB) direkt
benachbart, VOR dem VOR-Container"), das sollte für Kz-Elemente automatisch stimmen, solange
`kz_pfade()` dieselbe Schema-Reihenfolge liefert (laut Docstring Zeile 59-61 tut sie das). Für
`Vorsatz` (keine Kz) gilt das nicht — der bräuchte, weil außerhalb des Kz-Walks, eine eigene,
explizite Platzierung ans Ende von `E10`.

---

## Anhang: Geprüfte, aber ergebnislose Suchen (0 Treffer, exakter Suchbegriff)

- `iban` als Sachverhaltsfeld (nur PII-Maskierung: `produkt/haut/pii_filter.py`,
  `produkt/import/kontoauszug_writer.py:55,60,63`)
- `geburtsdatum` als eigenständiges Datumsfeld in `produkt/bindung/*.yaml` — nur als *Kohorten-
  Schlüssel-Ableitung* referenziert (`bindung_rentner.yaml:89,183,219`, `bindung_an_gesamt.yaml:141,837`),
  nicht als deklarierbares Feld
- `feld_id:.*konfession` außer der einen A-Zeile in `bindung_p51a_kirchensteuer.yaml:11`
- `veranlagungsart` (0 Treffer im ganzen `produkt/`)
- `kest`, `kapst`, `kap_est`, `kapitalertragsteuer_` als Feld-Präfix
- `E0203003`, `E0203501` außer in Testdateien/Fuzz-Test (keine Bindung)

## Vollständige Liste FEHLT_STAMMDATUM mit Feld-ID-Vorschlägen (26)

| Kz | Feld-ID-Vorschlag |
|---|---|
| AbsName | `stammdaten_name_a` (zusammengesetzt aus Vorname+Nachname A) |
| AbsOrt | spiegelt `stammdaten_ort` |
| AbsPlz | spiegelt `stammdaten_plz` |
| AbsStr | zusammengesetzt aus `stammdaten_strasse`+`stammdaten_hausnummer` |
| StNr | `stammdaten_steuernummer` |
| E0100201 | `stammdaten_nachname` |
| E0100301 | `stammdaten_vorname` |
| E0100401 | `stammdaten_geburtsdatum` |
| E0100402 | Abgleich mit bestehendem `kist_konfession` prüfen, ggf. kein neues Feld |
| E0100601 | `stammdaten_plz` |
| E0100602 | `stammdaten_ort` |
| E0100701 | `stammdaten_verheiratet_seit` |
| E0100801 | `stammdaten_vorname_partner` |
| E0100901 | `stammdaten_nachname_partner` |
| E0101001 | `stammdaten_geburtsdatum_partner` |
| E0101002 | `stammdaten_konfession_partner` (kein bestehendes Äquivalent) |
| E0101003 | `stammdaten_beruf_partner` |
| E0101004 | `stammdaten_titel` (optional) |
| E0101104 | `stammdaten_strasse` |
| E0101206 | `stammdaten_hausnummer` |
| E0101207 | `stammdaten_hausnummerzusatz` (optional) |
| E0102002 | `stammdaten_keine_bankverbindung` (oder IBAN-Erfassung neu bauen — Instructor-Entscheidung) |
| E0109109 | `an_sparzulage_beantragt` |
| E0200002 | `lstb_steuerklasse` |
| E1900601 | `kap_kist_pflicht_ohne_einbehalt` |
| E1904701/4801/4901 | `kap_steuerabzug_kapitalertragsteuer`/`_kist`/`_soli` (aus Steuerbescheinigung der Bank) |
