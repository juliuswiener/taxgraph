# Registry-Erstbefüllung — Stand nach Julius' Triage (2026-07-11)

## Umgesetzt

**Neue Manifest-Bedingungen** (aus der Triage, Anker gegen die eingefrorenen
Normen geprüft):
- § 33 Abs. 3: `kinder_sind_anspruchskinder` (Satz 2), `splitting_ist_veranlagungsergebnis`
  (Satz 1 Nr. 1 b) — Muster „Input ist Rechtsprädikat".
- § 9 Abs. 6: `erstausbildung_nach_legaldefinition` (Sätze 2–5).

Die anderen triagierten Items mappen auf **bereits vorhandene** Bedingungen (p24b
`alleinstehend_im_sinne_des_absatzes_3`, p9_4a `keine_unterbrechung_mit_neubeginn`,
p35a `dienstleistungsbetrag_enthaelt_nur_arbeitskosten`, nr5
`keine_verpflichtende_dienst_oder_werkswohnung`) — kein Duplikat nötig.

**§ 35a Andockpunkt** um die Reihenfolge-Aussage ergänzt: die Reihenfolge der
Ermäßigungen ist Aufgabe der § 2-Stufenfolge, nicht der Regel. Der A-Fund ist
`nicht_material`.

**§ 33 Registry gefüllt** (einzige Regel mit gültigen Ankern): 7 `bedingung_neu`-Items
(alle Referenz-Varianten von Kinder + Splitting) + 1 `defekt_formalisierer`
(ergebnis/rundung — der bekannte A-Fehler, koppelt an `freigabe: blockiert`).

**Wirkung, deterministisch belegt:** § 33 `geltungsbereich` und `roundtrip` sind von
FAIL (Stufe 3) auf **PASS** gegangen — die Registry deckt die Items ab. Es blockieren
nur noch `equivalence`, `clerk` und das neue `defekt`-Gate (alle drei: der
A-Rundungsfehler). Das ist die Ratsche end-to-end auf echten Daten.

**Zwei neue Triage-Status** gebaut und getestet: `defekt_formalisierer` (nie eine
Bedingung — der `aufnehmen`-Guard verweigert das), `offen_bis_neuschnitt` (Kern-
Anwendbarkeit, blockiert die zuschnitt_offen-Regel nicht).

## Was noch fehlt — und warum

Die Registries der **übrigen sechs aktiven Regeln** sind noch leer. Grund: ihre
gespeicherten Verdikte stammen aus **alten Judge-Templates** (`item_normteil@1`,
`roundtrip_diff@3`), die keine Anker tragen — fast alle `referenz` sind `?`,
`betrifft`/`kategorie` sind `?`/`sonstige`. Würde ich daraus seeden, gälte künftig
jedes Item wieder als neuer Fund; die Registry wäre wertlos.

**Diese sechs Regeln brauchen erst einen frischen `dekomponiert@2`-Judge-Lauf**
(`--redo-judge`, real, ~1,5 USD für alle sechs), damit die Verdikte echte Anker
tragen. Danach: `discover` → Julius' Triage aus diesem Paket auf die dann surfacenden
Items anwenden → `aufnehmen`. Wegen der Nicht-Sättigung surfacen nicht alle Items in
einem Lauf — das ist die Ratsche: über mehrere Läufe füllt sich die Registry.

Deshalb stehen die sechs Regeln aktuell auf `discovery_triage` (Manifest-Bedingungen
deklariert, aber noch nichts registriert, das sie bindet).

## Nach der Aufnahme erwartete verified_bedingt-Kandidaten

Sobald die Registries geseedet sind und die deterministischen Gates tragen:
**p24b, p10_1_7, p9_6, p9_1_3_nr5, p35a**. § 33 bleibt an der A-Blockade (defekt +
equivalence), nr5a/nr7 am Zuschnitt (`offen_bis_neuschnitt`).

## Nächster Schritt (braucht deine Freigabe für den Lauf)

Ein `--redo-judge` über die sechs Regeln (~1,5 USD), dann seede ich ihre Registries
aus deiner Triage und melde, welche `verified_bedingt` erreichen. Der Lauf kostet
Geld — deshalb deine Entscheidung, nicht meine.
