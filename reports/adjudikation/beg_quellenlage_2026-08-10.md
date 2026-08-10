# BEG-Heizungsförderung: Quellenlage 2026-08-10

Auftrag: Primärquellen für die BEG-Heizungsförderung (Reform ab 21.07.2026) beschaffen
und Belegstand für den geplanten Förderfinder feststellen. Reine Recherche/Beschaffung,
keine Berechnung, kein Code.

## Ergebnis in einem Satz

Die maßgebliche Richtlinienfassung (BEG EM vom 17.07.2026) wurde im Volltext gefunden und
lokal gesichert; **alle elf abgefragten Werte sind primärquellenbelegt und mit dem
KfW-Merkblatt 458 wortgleich cross-verifiziert.** Offen bleibt ausschließlich die formale
Bundesanzeiger-Zitierstelle (BAnz AT-Nummer) — die Richtlinie selbst führt sie noch als
Platzhalter, und sie war über keine erreichbare Quelle final auffindbar.

**Update Runde 2 (gleicher Tag):** ESanMV (Anspruchsvoraussetzung nach § 35c Abs. 1 S. 6
EStG) und BEG-Infoblatt zu den förderfähigen Maßnahmen zusätzlich beschafft und gesichert,
siehe Abschnitte weiter unten. Die "beihilferechtlich"-Frage beim Wertschöpfungsbonus bleibt
auch nach Prüfung des Infoblatts **unbelegt**.

## Beschaffte Dokumente

Alle Dateien liegen unter `sources/foerderung/` (Volltext .txt + Original .pdf + .meta.yaml
mit URL/Abrufdatum/SHA256, Muster wie `sources/bmf/`):

| Datei | Norm | Datum | Herkunft |
|---|---|---|---|
| `beg_em_foerderrichtlinie_2026-07-17` | BEG EM (Einzelmaßnahmen) | 17.07.2026 | energiewechsel.de (BMWE) |
| `beg_wg_foerderrichtlinie_2026-07-17` | BEG WG (Wohngebäude) | 17.07.2026 | energiewechsel.de (BMWE) |
| `beg_nwg_foerderrichtlinie_2026-07-17` | BEG NWG (Nichtwohngebäude) | 17.07.2026 | energiewechsel.de (BMWE) |
| `kfw458_merkblatt_2026-07` | KfW Merkblatt Zuschuss Nr. 458 | Stand 07/2026 | kfw.de |
| `gmodg_bgbl-2026-i-226_2026-07-28` | GModG (Regelungstext) | Gesetz v. 23.07.2026 | recht.bund.de |

**BEG EM ist die einschlägige Quelle** für die Heizungsförderung (Einzelmaßnahme
Heizungstausch); BEG WG regelt die Effizienzhaus-Gesamtsanierung, BEG NWG Nichtwohngebäude
— beide nur zur Vollständigkeit/Abgrenzung beschafft, nicht Grundlage der unten
tabellierten Werte.

Alle drei BEG-Richtlinien sind **Vorabfassungen**: unterzeichnet "Berlin, den 17. Juli 2026,
Bundesministerium für Wirtschaft und Energie, Im Auftrag Stephanie von Ahlefeldt", aber die
Bundesanzeiger-Fundstelle steht im Dokument selbst noch als Platzhalter
`(BAnz AT XX.XX.XXXX B1)`. Das KfW-Merkblatt 458 zitiert dieselbe energiewechsel.de-URL
explizit als "Anlage" — das ist die stärkste verfügbare Bestätigung, dass die abgerufene
Fassung tatsächlich die maßgebliche ist, ersetzt aber keine amtliche Fundstelle.

## Wertetabelle

| Wert | Belegt? | Fundstelle |
|---|---|---|
| Grundförderung 30 % | **Ja** | BEG EM Nr. 8.4.1 c): "Für Maßnahmen nach Nummer 5.3 beträgt der Fördersatz 30 %." Wortgleich KfW-Merkblatt 458 ("Als Grundförderung wird ein Zuschuss in Höhe von 30 Prozent … gewährt"). |
| Einkommensbonus dreistufig 40/30/10 % bis 30.000/40.000/50.000 € | **Ja** | BEG EM Nr. 8.4.5, exakt: "Bis 30 000 Euro … 40 Prozentpunkte", "Über 30 000 bis 40 000 Euro … 30 Prozentpunkte", "Über 40 000 bis 50 000 Euro … 10 Prozentpunkte". Wortgleich KfW-Merkblatt 458 Abschnitt "Einkommensbonus". |
| zvE gemittelt über 2. und 3. Jahr vor Antragstellung | **Ja** | BEG EM Nr. 3 Buchst. y) (Begriffsbestimmung "Zu versteuerndes Haushaltsjahreseinkommen"): "Durchschnitt aus den zu versteuernden Einkommen des zweiten und dritten Jahres vor Antragseingang". KfW-Merkblatt bestätigt sinngemäß, verweist zusätzlich auf ESt-Bescheide als Nachweis. |
| Familienzuschlag: mind. 1 minderjähriges Kind → −10.000 € Einkommensgrenze, einmalig | **Ja** | BEG EM Nr. 8.4.5 letzter Absatz: "…reduziert sich das anzusetzende zu versteuernde Haushaltsjahreseinkommen pauschal und einmalig um 10 000 Euro." Wortgleich KfW-Merkblatt ("Familienzuschlag", zusätzlich: Kindergeldberechtigung vorausgesetzt). |
| Klimageschwindigkeitsbonus 16/12/8/4 %, ab 01.08.2028 null | **Ja** | BEG EM Nr. 8.4.4, alle vier Halbjahresstufen wörtlich mit Datum: 21.07.2026–31.01.2027: 16 %-Punkte; 01.02.–31.07.2027: 12; 01.08.2027–31.01.2028: 8; 01.02.–31.07.2028: 4; "Ab 1. August 2028 entfällt der Bonus." Wortgleich KfW-Merkblatt 458. |
| Deckel 70 % / 80 % bei zvE ≤ 30.000 € | **Ja** | BEG EM Nr. 8.4.1 Einleitungssatz: "Obergrenze von 80 %… bis zu 30 000 Euro… und 70 %… über 30 000 Euro." Wortgleich KfW-Merkblatt ("Obergrenze von maximal 70 Prozent…", "80 Prozent" bei ≤30.000 €, inkl. 40.000-€-Variante mit Familienzuschlag). |
| Höchstbeträge 28.000 € / 15.000 € (2.–6. WE) / 8.000 € (ab 7. WE) | **Ja** | BEG EM Nr. 8.3.1 a): exakt diese drei Beträge. Wortgleich KfW-Merkblatt 458. |
| Absenkung Höchstbetrag 1. WE: ab 01.02.2027, halbjährlich −750 € | **Ja** | BEG EM Nr. 8.3.1 a) mit vollständiger Staffel-Tabelle bis "Ab 1. August 2030: 22 000 Euro". Wortgleich KfW-Merkblatt. |
| Entfallen: Effizienzbonus WP, Emissionsminderungszuschlag Biomasse, EE-Bonus | **Ja, indirekt** | Alle drei Begriffe kommen im vollständigen BEG-EM-Text **kein einziges Mal** vor (grep negativ), ebenso nicht im KfW-Merkblatt. Der Wegfall ist durch Abwesenheit in der neuen Fördersatz-Tabelle (Nr. 8.4.1) belegt — die Richtlinie enthält keine explizite Streichungsklausel "X entfällt", das ist Charakteristik einer Neufassung, nicht Textbefund. Sekundärquellen (BAFA-Kurzmeldung, reduco.ai) bestätigen den Wegfall explizit, decken sich mit dem Primärtextbefund. |
| Wertschöpfungsbonus 15 %-Punkte, EU-Wärmepumpen, ab Q1 2027, "geplant, nicht final" | **Teilweise** | Fördersatz und Zeitpunkt belegt: BEG EM Nr. 8.4.6: "wird ab Quartal 1 2027 zusätzlich ein Bonus von 15 Prozentpunkten gewährt, wenn die geförderte Wärmepumpe ihren Ursprung in der Union hat. Näheres regelt das 'Infoblatt zu den förderfähigen Maßnahmen und Leistungen'." **Nicht belegt**: der Zusatz "unter beihilferechtlichem Vorbehalt" — der Begriff "beihilfe" kommt im Richtlinientext nicht vor. Das KfW-Merkblatt 458 (Stand 07/2026) erwähnt den Wertschöpfungsbonus überhaupt nicht — konsistent mit "noch nicht in Kraft, wird erst zum Q1 2027 wirksam". Das erwähnte "Infoblatt zu den förderfähigen Maßnahmen und Leistungen", auf das die Richtlinie verweist, wurde nicht gesondert beschafft (nicht im Rechercheauftrag benannt, separates Dokument). |

**11 von 11 Werten sind primärquellenbelegt** (davon 10 direkt wörtlich, 1 — Wegfall der
drei Alt-Boni — indirekt über Abwesenheit im neuen Fördersatzkatalog). Der einzige nicht
belegte Teilaspekt ist die "beihilferechtlicher Vorbehalt"-Formulierung beim
Wertschöpfungsbonus, die in keiner der beschafften Primärquellen wörtlich auftaucht (der
Bonus selbst inkl. Satz und Zeitpunkt ist belegt).

## Richtlinienfassung: auffindbar?

**Ja, im Volltext** — aber als Vorabveröffentlichung durch das BMWE auf energiewechsel.de,
nicht als amtlich zitierfähige Bundesanzeiger-Bekanntmachung.

Gesucht wurde gezielt nach der BAnz-Fundstelle:
- WebSearch `"BAnz AT" BEG Wohngebäude Richtlinie 2026 Bekanntmachung`
- WebSearch `bundesanzeiger.de BEG Einzelmaßnahmen "17. Juli 2026" Bekanntmachung B1`
- WebSearch `"BAnz AT 20.07.2026" OR "BAnz AT 21.07.2026" BEG effiziente Gebäude`
- WebFetch auf `bundesanzeiger.de/pub/publication/xSizk6DUlWm93L4XrkY` (Treffer aus obiger
  Suche, Titel "Bundesförderung für effiziente Gebäude (BEG)") — schlug fehl
  ("Too many redirects", JS-Frontend nicht headless abrufbar)
- Direktabruf `bundesanzeiger.de/pub/de/amtlicher-teil?...&edition=BAnz+AT+21.07.2026` per
  curl — lieferte nur JS-Shell ohne Inhalt (SPA, kein serverseitig gerendertes HTML)

Keine dieser Routen ergab eine bestätigte "BAnz AT [Datum] [Buchstabe]"-Nummer für die
2026er-Fassung. Auch keine der gefundenen Sekundärquellen (tga-fachplaner.de,
bfw-newsroom.de, BAFA-Kurzmeldung) nennt eine Nummer — mehrere räumen selbst ein, dass die
Fundstelle zum Zeitpunkt ihrer Veröffentlichung noch offen war. Die drei Richtlinien selbst
tragen weiterhin den Platzhalter `(BAnz AT XX.XX.XXXX B1)` an der Stelle, wo sie auf ihre
eigene Fundstelle verweisen (BEG EM, Nr. 6.1, "Zuschusszusage… mit Datum vom 17. Juli 2026
(BAnz AT XX.XX.XXXX B1)").

Für den Förderfinder ist das **kein Blocker für die Werte selbst** (die sind über den
Amtstext + KfW-Merkblatt doppelt belegt), aber ein offener Punkt für eine später ggf.
gewünschte formal-zitierfähige Quellenangabe im Gesetzesregister-Stil (`BAnz AT …`
statt nur Datum + URL).

## GModG: Verkündungsstand

**Vollständig verkündet und in Kraft**, unabhängig von der offenen BAnz-Frage der
Förderrichtlinie:

- Bundestag/Bundesrat-Beschluss: 10.07.2026
- Ausfertigung: 23.07.2026 (Berlin; Steinmeier, Merz, Reiche, Hubertz)
- Verkündung: **BGBl. 2026 I Nr. 226, ausgegeben zu Bonn am 28.07.2026**
- Inkrafttreten Hauptteil: 29.07.2026 (Art. 9 Abs. 1: "am Tag nach der Verkündung");
  Art. 2+7 zum 01.01.2027, Art. 3 zum 01.01.2028, Art. 4 zum 01.01.2030

Wichtig für die Modellierung: **Das GModG regelt keine Fördersätze.** Es ist Ordnungsrecht
(löst die 65-%-EE-Pflicht des GEG ab, benennt das GEG faktisch um). Grep nach
`Fördersatz|Grundförderung|Einkommensbonus|BEG ` im vollständigen Regelungstext ergab keinen
Treffer — bestätigt die in mehreren Sekundärquellen behauptete Trennung: Förderhöhe kommt
ausschließlich aus der BEG-Richtlinie, nicht aus dem GModG.

## Was gesucht und nicht (vollständig) gefunden wurde

1. **BAnz-AT-Fundstelle der BEG-Richtlinien 2026** — siehe oben, trotz vier verschiedener
   Suchansätze nicht auffindbar, nur als Platzhalter im Dokument selbst vorhanden.
2. **"Unter beihilferechtlichem Vorbehalt"** beim Wertschöpfungsbonus — dieser Zusatz taucht
   in keiner der drei Primärquellen (BEG EM, BEG WG, BEG NWG, KfW-Merkblatt) wörtlich auf.
   Möglich, dass er aus dem "Infoblatt zu den förderfähigen Maßnahmen und Leistungen"
   stammt, auf das die Richtlinie verweist — dieses Infoblatt wurde nicht beschafft (lag
   außerhalb des ursprünglichen Rechercheauftrags, keine URL bekannt).
3. **Frühere/ältere Rechenbeispiele** wurden bewusst nicht verwendet — bei jedem
   Sekundärquellentreffer wurde auf Datumsangabe/Fassungsbezug geprüft; Treffer ohne klaren
   2026er-Reformbezug wurden verworfen, nicht zitiert.

## Einschätzung: Phase 1 baubar?

**Ja.** Alle elf abgefragten Zahlenwerte für den Kern-Rechenpfad (Grundförderung,
dreistufiger Einkommensbonus, Familienzuschlag, Klimageschwindigkeitsbonus mit allen vier
Zeitstufen, Deckel, Höchstbeträge samt Absenkungsstaffel, zvE-Mittelungsregel, Wegfall der
drei Altboni) liegen wörtlich aus der amtlichen Richtlinie vor und sind gegen das
KfW-Merkblatt 458 gegengeprüft — beide stimmen in jedem geprüften Punkt überein. Das ist eine
belastbare Grundlage für Code mit Parameterwert-Beleg-Pflicht.

Zwei Einschränkungen für den weiteren Bau:
- Die formale Zitierstelle (BAnz AT) fehlt noch; falls das Projekt eine belastbare
  Gesetzeszitierung im NeuRIS-/BGBl-Stil verlangt (vgl. `sources/README.md`), ist das ein
  offener Nachzieh-Punkt, kein Wert-Risiko.
- Der Wertschöpfungsbonus ist ohnehin laut Auftraggeber "geplant, nicht final" — für Phase 1
  ohnehin nicht scharf zu schalten; die "beihilferechtlich"-Einschränkung sollte bis zur
  Beschaffung des Infoblatts nicht als belegter Fakt in den Code, sondern als offene Notiz
  behandelt werden.

## Runde 2 (2026-08-10): ESanMV, BEG-Infoblatt, Bescheinigungsmuster

Nachtrag zum obigen Bericht, Auftrag: ESanMV dringend (Anspruchsvoraussetzung), Infoblatt
zur Maßnahmenliste, Bescheinigungsmuster nachrangig/optional.

### ESanMV (Energetische Sanierungsmaßnahmen-Verordnung)

**Gefunden und vollständig gesichert.** Rechtsverordnung nach § 35c Abs. 7 EStG —
§ 35c Abs. 1 S. 6 EStG macht "die Anforderungen aus der Rechtsverordnung nach Absatz 7"
zur **Tatbestandsvoraussetzung** der Steuerermäßigung, nicht nur zu einem Betragsparameter.
Ohne diese Verordnung kann der Förderfinder zwar Beträge rechnen, aber nicht die
Anspruchsberechtigung prüfen — daher die von Auftraggeber markierte Dringlichkeit.

- Quelle: `gesetze-im-internet.de/esanmv` (Bundesjustizministerium/juris), Volltext §§ 1-4 +
  Anlagen 1-8.
- Fundstellenkette: Ausfertigung 02.01.2020 (BGBl. I S. 3); letzte Änderung Art. 1 der
  Verordnung vom 4.11.2024 (BGBl. 2024 I Nr. 341) — betrifft insb. Anlage 4a (neu), Anlage 6
  (Heizungsanlagen, komplett neu gefasst) und Anlage 7 (digitale Systeme).
- **§ 3 ESanMV (Anwendungsregelungen), primärquellenbestätigt**: "Diese Fassung der
  Verordnung ist erstmals für den Veranlagungszeitraum 2025 anzuwenden; sie gilt für
  energetische Maßnahmen, mit denen nach dem 31. Dezember 2024 begonnen wurde." Löst damit
  einen zuvor offenen Punkt (VZ-2025-Anwendbarkeit) direkt aus dem Verordnungstext.
- § 4: Stand-Vermerk im Original nennt sowohl ein Außerkrafttreten der Verordnung zum
  31.12.2029 als auch, für § 4 selbst, einen Termin 1.1.2030 (laut Art. 1 Nr. 4 der
  Verordnung vom 14.6.2021) — beide Angaben stehen so nebeneinander im Originaltext, kein
  Abschreibfehler beim Sichern.
- Extraktionsmethode bewusst PDF statt HTML: `scripts/freeze_source.py` wurde **nicht**
  verwendet, weil sein generisches HTML-Tag-Stripping die Spaltenstruktur der U-Wert-Tabellen
  (Anlagen 1-4) und der Wärmepumpen-Effizienzkennzahlen (Anlage 6) zerstört hätte. Stattdessen
  `pdftotext -layout` auf das offizielle PDF; die HTML-Fassung wurde stichprobenartig
  gegengelesen, kein inhaltlicher Unterschied in §§ 1-4 gefunden.
- Datei: `sources/gesetze-im-internet/esanmv_2026-08-10.{txt,pdf,meta.yaml}`,
  `einfrier_ebene: "ganze Verordnung"` (nicht nach Paragraph aufgeteilt, da § 35c Abs. 7 EStG
  die Anlagen als Gesamtkatalog referenziert und diese untrennbar von §§ 1/3 sind).

### BEG-Infoblatt zu den förderfähigen Maßnahmen und Leistungen – Sanieren

**Gefunden und gesichert**, aber mit einer für den Förderfinder wichtigen Einschränkung.

- Quelle: bafa.de, gemeinsames Dokument von BAFA und KfW, Version 10.1, Bestellnummer
  600 000 4863. Datei: `sources/foerderung/beg_infoblatt_2026-08-10.{txt,pdf,meta.yaml}`.
- Quellenrang wie beim KfW-Merkblatt 458: Programmträger-/Vollzugsdokument, kein Normgeber.
  Inhaltlich zu **Kostenarten** je Maßnahme (was zählt als förderfähige Aufwendung), nicht zu
  Fördersätzen/Boni — für Letztere bleibt die Förderrichtlinie selbst maßgeblich.
- **Kritischer, selbst-deklarierter Befund im Dokument** (bestätigt den "sei
  skeptisch"-Auftrag konkret): Version 10.1 hat laut eigenem Vorwort **ausschließlich** die
  Definition Worst-Performing-Building (Abschnitt 1.6) an die 2026er-Reform angepasst; alle
  übrigen Abschnitte (2-9, insb. die Maßnahmenliste in Abschnitt 4) werden laut Dokument
  "derzeit umfassend überarbeitet", eine vollständig aktualisierte Fassung sei erst "in
  Kürze" zu erwarten und existierte zum Abrufzeitpunkt noch nicht. Für den Förderfinder heißt
  das: die Gliederung/Kategorienliste ist als Strukturgrundlage brauchbar, aber jedes Detail
  darin muss gegen die Förderrichtlinie selbst gegengeprüft werden, bevor es als
  reform-aktuell gilt.
- **Negativbefund** (Runde-1-Hypothese nicht bestätigt): Volltextsuche nach "beihilf" und
  "wertschöpfung" ergab in diesem Dokument **keinen Treffer**. Die Vermutung, dieses
  Infoblatt sei die Quelle für den beihilferechtlichen Vorbehalt beim Wertschöpfungsbonus,
  hat sich nicht bestätigt — das Infoblatt behandelt Kostenarten, keine Boni/Vorbehalte. Die
  Formulierung bleibt weiterhin unbelegt.
- **Positivbefund**: Abschnitt 4.3 "Voraussetzungen für den Klimageschwindigkeits-Bonus"
  referenziert wörtlich "Maßnahmen nach Nummer 5.3 der Förderrichtlinie BEG EM" — identische
  Nummerierung wie in der bereits archivierten Richtlinie
  `beg_em_foerderrichtlinie_2026-07-17`. Ein Indiz (kein Beweis), dass zumindest dieser
  Abschnitt bereits reform-aktuell ist, trotz des pauschalen Nicht-Aktualitäts-Hinweises für
  das Gesamtdokument.

### Bescheinigungsmuster nach § 35c Abs. 1 S. 7 EStG (nachrangig, Zusatzfund)

Explizit als nachrangig markiert ("nicht lange danach suchen") — ohne größeren Suchaufwand
gefunden (eine WebSearch, eine Landingpage, zwei Direktdownloads) und daher mitgesichert:

- **BMF-Schreiben vom 23.12.2024** (GZ IV C 1 - S 2296-c/20/10003 :008): amtliches Muster der
  Bescheinigung des ausführenden Fachunternehmens bzw. der nach § 88 GEG
  ausstellungsberechtigten Person. Ersetzt BMF-Schreiben v. 26.01.2023 (BStBl I S. 218),
  geändert durch BMF-Schreiben v. 06.02.2024 (BStBl I S. 237) — seit 01.01.2025 ein
  einheitliches Muster (vorher getrennte Muster I/II). Gilt für Maßnahmenbeginn nach
  31.12.2024; für früheren Beginn verweist das Schreiben auf die jeweiligen Vorgänger-Muster
  (2020/2021/2023).
- Rn. 12 des Schreibens bestätigt den Bezug zur ESanMV-Änderung: "Für die bescheinigten
  Maßnahmen müssen die Mindestanforderungen der zum 1. Januar 2025 durch die Dritte
  Verordnung zur Änderung der Energetische Sanierungsmaßnahmen-Verordnung vom 4. November
  2024 (BGBl. I Nr. 341) angepassten ESanMV eingehalten werden." — dieselbe Änderungsverordnung
  wie oben bei der ESanMV selbst gefunden, gegenseitig konsistent.
- Zwei Dateien gesichert: `sources/bmf/bmf_35c_musterbescheinigung_2024-12-23.{txt,pdf,meta.yaml}`
  (das Schreiben inkl. eingebettetem Formular) und
  `sources/bmf/bmf_35c_musterbescheinigung_formular_2024-12-23.{txt,pdf,meta.yaml}`
  (identischer Formular-Inhalt als eigenständige BMF-Publikation).
- Charakter: Verwaltungsauffassung (`authority: verwaltung`), kein Gesetzestext — für den
  Förderfinder relevant als Referenz-Katalog der über § 35c EStG bescheinigungsfähigen
  Maßnahmen-Kategorien, nicht für die BEG-Fördersätze.

## Dateiliste

```
sources/foerderung/beg_em_foerderrichtlinie_2026-07-17.{txt,pdf,meta.yaml}
sources/foerderung/beg_wg_foerderrichtlinie_2026-07-17.{txt,pdf,meta.yaml}
sources/foerderung/beg_nwg_foerderrichtlinie_2026-07-17.{txt,pdf,meta.yaml}
sources/foerderung/kfw458_merkblatt_2026-07.{txt,pdf,meta.yaml}
sources/foerderung/gmodg_bgbl-2026-i-226_2026-07-28.{txt,pdf,meta.yaml}
sources/foerderung/beg_infoblatt_2026-08-10.{txt,pdf,meta.yaml}
sources/gesetze-im-internet/esanmv_2026-08-10.{txt,pdf,meta.yaml}
sources/bmf/bmf_35c_musterbescheinigung_2024-12-23.{txt,pdf,meta.yaml}
sources/bmf/bmf_35c_musterbescheinigung_formular_2024-12-23.{txt,pdf,meta.yaml}
```

Alle SHA256-Hashes in den `.meta.yaml`-Dateien wurden gegen die tatsächlichen Dateien
gegengeprüft (ein Transkriptionsfehler beim KfW-PDF-Hash wurde dabei gefunden und korrigiert
[Runde 1]; alle Runde-2-Hashes wurden nach Berechnung auf 64-stellige Hex-Länge geprüft).
