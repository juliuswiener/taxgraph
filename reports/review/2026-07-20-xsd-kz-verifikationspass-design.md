# XSD-Kz-Verifikationspass — Design (dev-2, 2026-07-20)

Design-first, kein Bau. Reiner READ-ONLY-Report-Baustein: bindung-Feld → XSD-Kz-Existenz +
Sektionspfad verifizieren, NIE auto-editieren. Adressiert dev-3s 5 False-Positive-Constraints,
inkl. der nachgelagerten Soundness-Frage (Element-Namen-Kette vs. Typ-Namen-Reverse-Lookup).
Alle Aussagen unten sind empirisch am echten lokalen ERiC-Schema geprüft (Prototyp
`/tmp/xsd_proto/walker.py`, nicht committet — Wegwerf-Beweis-Code für dieses Design).

## 1. Die Soundness-Frage zuerst (dev-3s kritischer Einwand)

**Frage:** Baut der Sektions-Pfad-Walk aus LOKALEN xs:element-Namen (pro Hop: Element-Name +
umschließender Typ) — oder reverse-indext er über TYP-NAMEN (welches Element referenziert
diesen Typ)?

**Antwort: lokale Element-Namen-Kette, TOP-DOWN, keine type→path-Memo-Tabelle.**

Begründung (nicht nur Behauptung — Gegenbeweis unten): ein benannter complexType kann von 2+
verschiedenen Elementen referenziert werden (im echten E10-Schema empirisch bestätigt — siehe
§4). Eine Reverse-Lookup-Tabelle „Typ X → Pfad" ist dann bei Wiederverwendung MEHRDEUTIG
(welcher der referenzierenden Pfade ist „der" Pfad?). Ein TOP-DOWN-Walk ab der Schema-Wurzel
hat dieses Problem strukturell nicht: jede xs:element-Referenzstelle wird EINZELN besucht,
mit ihrer eigenen Ahnen-Kette aus lokalen Namen — der referenzierte complexType wird nur zum
Reinschauen (welche Kind-Elemente hat er) benutzt, nie um "den" Pfad eines Typs nachzuschlagen.
Wird derselbe Typ an 2 Stellen referenziert, entsteht dadurch AUTOMATISCH 2 getrennte
Baum-Besuche mit 2 unterscheidbaren Pfaden — keine Kollision möglich.

### Gegenbeweis: synthetisches Schema mit bewusst geteiltem complexType

```xml
<xs:element name="Envelope" type="EnvelopeType"/>
<xs:complexType name="EnvelopeType">
  <xs:sequence>
    <xs:element name="Absender" type="AdresseType"/>
    <xs:element name="Empfaenger" type="AdresseType"/>   <!-- GETEILTER Typ -->
  </xs:sequence>
</xs:complexType>
<xs:complexType name="AdresseType">
  <xs:sequence>
    <xs:element name="E9999001" type="xs:string"/>
  </xs:sequence>
</xs:complexType>
```

Walker-Ergebnis für Kz `E9999001`:
```
Envelope/Absender/E9999001
Envelope/Empfaenger/E9999001
```
2 Fundstellen, beide korrekt und unterscheidbar — `AdresseType` wird zweimal unabhängig
durchlaufen, nie als "ein" Pfad gecacht. (Ein Typ-Namen-Reverse-Index hätte hier `AdresseType
→ ?` beantworten müssen und wäre bei genau diesem Muster strukturell mehrdeutig.)

## 2. Algorithmus (Pseudocode, entspricht dem geprüften Prototyp)

```
walk(root_xsd, start_element_name, kz_pattern):
    type_index  = { ct.name: ct  for ct in alle complexType-Definitionen }   # nur zum Reinschauen
    group_index = { g.name: g    for g in alle xs:group-Definitionen }       # defensiv, s. §5
    found = []

    recurse(elem_node, path, depth):
        if depth > MAX_DEPTH: return                      # Zyklen-Backstop, empirisch nie nötig (§4)
        local_name = elem_node.name (oder .ref)
        new_path = path + [local_name]
        if kz_pattern(local_name):
            found.append((new_path, local_name))           # JEDE Fundstelle einzeln, keine Dedup

        content = inline complexType  ODER  type_index[elem_node.type]
        if content is None: return                          # Blatt (primitiver Typ) -> Ende

        for child in content_children(content):             # sequence/choice/all rekursiv geflacht
            if child ist xs:element:      recurse(child, new_path, depth+1)
            if child ist xs:group-ref:    für jedes Kind-Element in group_index[ref]: recurse(...)

    start = top-level xs:element mit start_element_name
    recurse(start, [], 0)
    return found                                             # [(pfad_liste, kz_id), ...]
```

Verifikations-Report (die eigentliche Anwendung, kein Auto-Edit):
```
für jedes bindung-Feld mit elster_kz != null:
    kandidaten = [ (pfad, kz) in walk(...) if kz == bindung.elster_kz ]
    wenn len(kandidaten) == 0:  -> FINDING "Kz nicht im Schema gefunden" (Jahr/Datenart prüfen)
    wenn len(kandidaten) == 1:  -> OK, Pfad zur Doku loggen
    wenn len(kandidaten) > 1:   -> FAIL LOUD "mehrdeutig, kein Auto-Pick" (Mensch entscheidet)
```

## 3. dev-3s 5 Constraints — Status

**(1) Sektions-Pfad statt E-Nr-Präfix:** erledigt durch den Walk selbst (§1/§4) — keine
String-Heuristik auf der Kz-Nummer, reine Baum-Ahnenschaft.

**(2) Datenart-Scoping:** nur `Datenarten/ElsterErklaerung/ESt` (E10). Bestätigt: 895 lokale
XSDs insgesamt, alle 5 aktuellen `bindung_*.yaml`-Dateien referenzieren ausschließlich E10-Kz
(EStG-only) — GewSt/USt/KSt/EÜR/etc. sind für diesen Pass irrelevant, kein Scan nötig.

**(3) VZ-Jahr-Schema:** kein neues Schema-Feld — reuse `vz_gueltigkeit` (bereits pro
bindung-Eintrag vorhanden). Lokal verfügbare Jahre: 2020–2025 (alle 6 empirisch geprüft, siehe
§4). VZ2026 explizit als „unverifizierbar, kein lokales Schema" melden (ERiC 44.2.4.0 datiert
vor VZ2026-Veröffentlichung) — NIE still überspringen.

**(4) xs:include/xs:import auflösen:** generisch vorgesehen (Parser löst includes/imports auf,
bevor der Typ-Index gebaut wird), aber empirisch für E10-<Jahr>.xsd über ALLE 6 Jahre NICHT
gebraucht — jede Datei ist ein einziges self-contained `<xs:schema>` ohne `xs:include`/
`xs:import` (siehe §4-Tabelle). Baue die Auflösung trotzdem defensiv (kein blinder Fleck falls
sich das in einem Datenart/Jahr ändert), aber sie ist toter Code für den aktuellen Datenbestand.

**(5) legitime no-Kz-Felder ausschließen:** automatisch — jeder bindung-Eintrag mit
`elster_kz: null` trägt bereits ein dokumentiertes `elster_kz_grund` und wird vom Report
schlicht nie nachgeschlagen (kein Sonderfall nötig).

## 4. Empirische Belege (echtes lokales ERiC-Schema)

**Root-Element + include/import/group je Jahr** (alle 6 Jahre identisch geprüft):

| Jahr | Root-Element | xs:include | xs:import | xs:group-Refs | xs:attributeGroup-Refs |
|------|-------------|-----------|-----------|----------------|-------------------------|
| 2020–2025 | `E10` (Typ `E10_CType`) | 0 | 0 | 0 | 0 |

**K_Verh_A/B-Probe (die ursprüngliche Motivation) — Walker-Resultat gegen `E10-2025.xsd`:**
```
E0500406 -> E10/Kind/Ang_Kind/Allg/E0500406
E0500702 -> E10/Kind/Ang_Kind/Allg/E0500702
E0500807 -> E10/Kind/K_Verh/K_Verh_A/E0500807
E0500808 -> E10/Kind/K_Verh/K_Verh_B/E0500808
```
Reine lokale Namensketten, keine Typ-Namen im Spiel — Ergebnis deckt sich mit dem bereits
gefreezten `bindung_*`-Katalog.

**Laufzeit/Größe:** 2242 Kz-Fundstellen im gesamten E10-2025.xsd, 11ms, max. Pfadtiefe 8
(MAX_DEPTH-Backstop von 80 nie in Reichweite — kein Zyklenrisiko im echten Schema).

**Geteilte complexTypes real vorhanden:** 78 Typen in E10-2025.xsd werden von 2+ verschiedenen
Elementen referenziert (`Ja1BaseCType_RABE`, `String_MinL1_MaxL999_CType_RABE`, etc.) — ALLE
78 sind reine Blatt-/Kodierungs-Typen ohne eigene Kind-Elemente (geprüft: 0 von 78 hat
verschachtelte `xs:element`). Für diese Datei ist „geteilter STRUKTURELLER Typ" also aktuell
kein beobachtbarer Fall — die Soundness-Absicherung (§1) ist trotzdem nötig, weil sie nicht von
dieser Zufallseigenschaft der aktuellen Schema-Generierung abhängen darf.

**Generalisierung über K_Verh_A/B hinaus (Person A/B, dev-3s Frage):** geprüft an den
gefreezten `bindung_rentner.yaml`-Einträgen (E0109708/706/704/E0161606/808 = Person A,
E0505809/807 = Person B/Ehegatte). Befund: Person A und Person B sind im Schema NICHT
dieselbe Kz-Nummer unter zwei `_A`/`_B`-Geschwister-Blöcken (anders als K_Verh_A/B) — es sind
von vornherein VERSCHIEDENE Kz-IDs unter komplett getrennten Hash-benannten Blöcken
(`66196332` vs. `m1740292092`). D.h. der Walk braucht hier gar keinen Ambiguitäts-Gate: jede
Kz-ID hat genau einen Pfad, weil Person A/B strukturell nie dieselbe Kz-Nummer teilen. Über das
gesamte E10-Schema geprüft: **0 Kz-IDs kommen an >1 unterscheidbarem Pfad vor** (2242
Fundstellen, alle mit eindeutiger Kz→Pfad-Zuordnung). Der „mehrdeutig→FAIL LOUD"-Zweig in §2
ist damit ein Backstop für einen Fall, der im aktuellen E10-Datenbestand nicht auftritt, aber
strukturell möglich bleibt (z.B. ein künftiger Datenart/Jahr-Sonderfall) — bewusst NICHT
weggelassen. Offener Punkt bleibt (unverändert zu [[kz-block-disambiguierung-personA-B]]):
Ehegatte-Zuordnung für neue/unbekannte Felder ist bei GLEICHER Kz-Nummer auf beiden Seiten
weiterhin ein GAP — kommt im XSD selbst aber laut obigem Befund praktisch nicht vor.

## 5. Aufwands-Neubewertung

Vor diesem Design: grobe Schätzung ~3h (unsicher, weil Soundness offen).
Nach diesem Design: das technisch riskante Kernstück (Walk-Algorithmus, Soundness-Beweis,
Jahres-/Include-Empirie) ist bereits fertig UND geprüft (~90 Zeilen Prototyp, oben). Verbleibt
reine Klebearbeit: `traverser.lade_bindung()` wiederverwenden (existiert bereits), Walker in
`produkt/mapping/` oder eigenes Skript einbauen, Report-Format (OK/AMBIGUOUS/NOT_FOUND je
Feld), Tests (real-Schema + der synthetische Geteilt-Typ-Fall aus §1 als Regressions-Fixture).
**Revidierte Schätzung: ~1–1.5h**, kein Grund zum Defern.

## 6. Hardening-Auflagen (dev-3, Build-Freigabe 2026-07-20)

Build-Go erteilt, mit 3 verpflichtenden Härtungen + 2 Doku-Notizen, alle in den Pseudocode/Plan
unten eingearbeitet (kein Scope-Zuwachs — reine Konsequenz aus §1-§4):

**(H1) Report-Summary MUSS Gate-tauglich sein.** Ein "lauter" Fund darf nie nur im Log stehen —
der Report-Treiber (der dünne Layer über `walk()`, der pro bindung-Feld klassifiziert) gibt am
Ende einen Exit-Code zurück:
```
exit 0   nur wenn ALLE Felder OK
exit 1   sobald mind. 1 Feld AMBIGUOUS oder NOT_FOUND ist
```
Damit ist der Pass in ein CI-/pytest-Gate einhängbar, ohne dass jemand das Log lesen muss.

**(H2) Die §1-Regressions-Fixture MUSS durch den VOLLEN Report-Treiber laufen, nicht nur durch
`walk()`.** Non-vacuous-Disziplin wie beim conf_map-Fund: Prüfgegenstand ist die
KONSUMENTEN-Klassifikation, nicht die Walker-Primitive. Geplanter Test (Bau-Phase):
```
synthetic_schema = das Envelope/Absender/Empfaenger/AdresseType-Schema aus §1
bindung_test = {"feld_x": {"elster_kz": "E9999001", ...}}
ergebnis = report_treiber(synthetic_schema, bindung_test, start_element="Envelope")
assert ergebnis["feld_x"].status == "AMBIGUOUS"     # NICHT nur: walk() liefert 2 Pfade
assert report_treiber_exit_code(ergebnis) == 1       # H1-Kopplung: Gate schlägt wirklich an
```
Ein Test, der nur `len(walk(...)) == 2` prüft, wäre vacuous (das ist die Walker-Primitive, nicht
der Konsument) — genau der Fehler, der beim ersten conf_map-Test schon einmal passiert ist.

**(H3) MAX_DEPTH-Abbruch MUSS gezählt + im Report-Summary sichtbar sein.** Aktuell (Prototyp
§2) ist `if depth > MAX_DEPTH: return` ein stiller Abbruch — inkonsistent mit der eigenen
Fail-Loud-Philosophie (§2, `> 1 Kandidat` wird laut, aber "Baum zu tief, evtl. Kz verpasst"
bisher nicht). Fix im Bau: `recurse()` zählt jeden MAX_DEPTH-Abbruch in einen
`max_depth_hits`-Zähler; Report-Summary zeigt ihn IMMER (auch wenn 0 — Transparenz statt
Stille). Bei `max_depth_hits > 0` zusätzlich `exit 1` (H1) — ein abgebrochener Teilbaum kann
einen Kz-Fund verpasst haben, das ist strukturell dasselbe Risiko wie NOT_FOUND.

**Doku-Note 1 — `xs:attributeGroup`:** anders als `xs:group` (defensiv aufgelöst, §3 Punkt 4)
braucht `xs:attributeGroup` KEINE Auflösung — strukturell ausgeschlossen, nicht nur empirisch
leer: `xs:attributeGroup` kann ausschließlich `xs:attribute`-Knoten einführen, nie
`xs:element`-Kinder. Da jeder Kz in diesem Schema als Element modelliert ist (verifiziert: 148
`xs:attribute`-Knoten insgesamt in E10-2025.xsd, keiner davon Kz-benannt), kann eine
`attributeGroup`-Indirektion strukturell NIE einen Kz-Fund erzeugen oder verstecken — kein
Blindfleck, sondern eine kategorische Nicht-Anwendbarkeit. (Zusätzlich empirisch bestätigt: 0
`attributeGroup`-Verwendungen über alle 6 Jahre.)

**Doku-Note 2 — `substitutionGroup`:** 0 Verwendungen über alle 6 Jahre (2020–2025, verifiziert
per grep). Low-Risk, kein Sonderfall im Bau vorgesehen — falls ein künftiges Schema-Jahr
`substitutionGroup` einführt, würde ein per Referenz eingebundenes Element unter dem Namen des
SUBSTITUENTEN (nicht des abstrakten Kopf-Elements) im Baum auftauchen; der Walk sieht das
automatisch korrekt, sofern das substituierende Element normal per `ref=` eingebunden ist —
träfe aber nicht das defensive `xs:group`-Auflösungs-Muster, sondern bräuchte eine eigene
Prüfung, falls es je auftritt (aktuell: totes Szenario, nicht gebaut).

## 7. Referenzen

[[kz-block-disambiguierung-personA-B]] · [[dba-anker-nur-amtlich]] (amtliche Quelle für
Schema-Datei-Wahl, hier: lokales ERiC 44.2.4.0) · Prototyp nicht committet
(`/tmp/xsd_proto/`), auf Wunsch reproduzierbar (kein externer Zustand).
