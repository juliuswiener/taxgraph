# Formalisierungs-Backlog (aus scope_gaps)

Automatisch erzeugt aus den Judge-Verdikten der Gate-Kaskade. Enthalten sind ausschliesslich Norm-Teile der Klasse `unabhaengig`: sie liegen ausserhalb der formalisierten Signatur und aendern deren Ergebnis nicht. Norm-Teile, die in den Signatur-Scope hineinwirken, stehen NICHT hier - sie eskalieren im `scope_gap`-Gate, weil der Ausschnitt ohne sie falsch waere.

21 Item(s) aus 7 Regel(n).


## aus `p10_1_7_berufsausbildung`

- **Bei Ehegatten, die die Voraussetzungen des § 26 Absatz 1 Satz 1 erfüllen, gilt Satz 1 für jeden Ehegatten.**
  - Die Regelung betrifft die Anwendung des Höchstbetrags bei zusammenveranlagten Ehegatten, ändert aber nicht den Höchstbetrag selbst oder die Berechnung für einen einzelnen Steuerpflichtigen, die der Scope modelliert.
- **Zu den Aufwendungen im Sinne des Satzes 1 gehören auch Aufwendungen für eine auswärtige Unterbringung.**
  - Diese Erweiterung des Aufwendungsbegriffs liegt außerhalb des Scopes, der nur einen bereits ermittelten Geldbetrag als Input erhält; sie verändert die Kappungsregel nicht.
- **§ 4 Absatz 5 Satz 1 Nummer 6b und 6c sowie § 9 Absatz 1 Satz 3 Nummer 4 und 5, Absatz 2, 4 Satz 8 und Absatz 4a sind bei der Ermittlung der Aufwendungen anzuwenden.**
  - Die Verweise betreffen die Ermittlung der abziehbaren Aufwendungen, die der Scope als gegeben voraussetzt; sie wirken sich nicht auf die Höchstbetragsregelung selbst aus.

## aus `p24b_entlastungsbetrag`

- **§ 24b Abs. 1 EStG: „wenn zu ihrem Haushalt mindestens ein Kind gehört, für das ihnen ein Freibetrag nach § 32 Absatz 6 oder Kindergeld zusteht“ sowie die Meldepflicht-Regelung**
  - Regelt, welche Kinder zählen, und liegt außerhalb der Signatur, die nur die Anzahl der Kinder als gegeben annimmt. Die Berechnungslogik des Entlastungsbetrags bleibt davon unberührt.
- **§ 24b Abs. 3 EStG: Definition „allein stehend“ (Splitting-Verfahren, verwitwet, Haushaltsgemeinschaft mit Ausnahme)**
  - Definiert den booleschen Input ‚alleinstehend‘, der außerhalb der Signatur bestimmt wird. Die Berechnung des Betrags aus den Inputs wird dadurch nicht verändert.

## aus `p33_3_zumutbare_belastung`

- **§ 33 Abs. 1 EStG: "Erwachsen einem Steuerpflichtigen zwangsläufig größere Aufwendungen ... so wird auf Antrag die Einkommensteuer dadurch ermäßigt, dass der Teil der Aufwendungen, der die dem Steuerpflichtigen zumutbare Belastung (Absatz 3) übersteigt, vom Gesamtbetrag der Einkünfte abgezogen wird."**
  - Regelt den Abzug der außergewöhnlichen Belastungen und die Rolle der zumutbaren Belastung als Kürzungsbetrag, nicht deren Berechnung. Die Berechnung der zumutbaren Belastung selbst bleibt unverändert.
- **Tabelle in § 33 Abs. 3 Satz 1 EStG: "bei Steuerpflichtigen, die keine Kinder haben und bei denen die Einkommensteuer a) nach § 32a Absatz 1, ... b) nach § 32a Absatz 5 oder 6 (Splitting-Verfahren) zu berechnen ist"**
  - Bestimmt, wann der Splitting-Tarif anzuwenden ist, also den Input 'splitting'. Die Berechnung der zumutbaren Belastung aus diesem Input ist davon nicht betroffen.
- **§ 33 Abs. 3 Satz 2 EStG: "Als Kinder des Steuerpflichtigen zählen die, für die er Anspruch auf einen Freibetrag nach § 32 Absatz 6 oder auf Kindergeld hat."**
  - Definiert den Kreis der zu berücksichtigenden Kinder und damit den Input 'anzahl_kinder'. Die Berechnung der zumutbaren Belastung aus einer gegebenen Kinderzahl ändert sich dadurch nicht.

## aus `p35a_2_3_haushaltsnahe`

- **Abs. 2 S.1: „Für andere als in Absatz 1 aufgeführte haushaltsnahe Beschäftigungsverhältnisse oder für die Inanspruchnahme von haushaltsnahen Dienstleistungen, die nicht Dienstleistungen nach Absatz 3 sind“**
  - Die Vorschrift grenzt lediglich den Kreis der begünstigten Aufwendungen ein, ändert aber nicht den Berechnungsmodus (20 %, Höchstbetrag) für die im Scope modellierten Aufwendungen.
- **Abs. 2 S.2: Einbeziehung von Pflege- und Betreuungsleistungen sowie Heimunterbringungskosten, soweit sie haushaltsnahen Dienstleistungen vergleichbar sind**
  - Die Erweiterung des Aufwendungsbegriffs wirkt sich nur auf die Zusammensetzung des Inputs aus; die Berechnung der Steuerermäßigung aus diesem Input bleibt unverändert.

## aus `p9_1_3_nr5_doppelte_haushaltsfuehrung`

- **Aufwendungen für die Wege vom Ort der ersten Tätigkeitsstätte zum Ort des eigenen Hausstandes und zurück (Familienheimfahrt) können jeweils nur für eine Familienheimfahrt wöchentlich abgezogen werden.**
  - Betrifft nur Familienheimfahrten, nicht die Unterkunftskosten; die Ausgabe des Scopes bleibt unverändert.
- **Zur Abgeltung der Aufwendungen für eine Familienheimfahrt ist eine Entfernungspauschale von 0,38 Euro für jeden vollen Kilometer der Entfernung zwischen dem Ort des eigenen Hausstandes und dem Ort der ersten Tätigkeitsstätte anzusetzen.**
  - Regelt die Pauschale für Familienheimfahrten, ohne Einfluss auf die Unterkunftskosten.
- **Nummer 4 Satz 3 bis 5 ist entsprechend anzuwenden.**
  - Verweist auf Regelungen zu Fahrten zwischen Wohnung und Tätigkeitsstätte, die nur für Familienheimfahrten gelten; keine Auswirkung auf Unterkunftskosten.
- **Aufwendungen für Familienheimfahrten mit einem dem Steuerpflichtigen im Rahmen einer Einkunftsart überlassenen Kraftfahrzeug werden nicht berücksichtigt.**
  - Schließt Abzug für bestimmte Familienheimfahrten aus; betrifft nicht die Unterkunftskosten.

## aus `p9_4a_verpflegungsmehraufwand`

- **Mehraufwendungen des Arbeitnehmers für die Verpflegung sind nur nach Maßgabe der folgenden Sätze als Werbungskosten abziehbar.**
  - Rein deklaratorische Einleitung, die den Anwendungsbereich umreißt, aber keine eigenständige, den Pauschalbetrag ändernde Regelung enthält.
- **Hat der Arbeitnehmer keine erste Tätigkeitsstätte, gelten die Sätze 2 und 3 entsprechend.**
  - Die Regelung stellt nur klar, dass die Berechnung auch ohne erste Tätigkeitsstätte anwendbar ist, ändert aber die Höhe der Pauschale nicht.
- **Eine Unterbrechung der beruflichen Tätigkeit an derselben Tätigkeitsstätte führt zu einem Neubeginn, wenn sie mindestens vier Wochen dauert.**
  - Die Regelung betrifft die Ermittlung des Eingabewerts 'monate_am_ort', nicht die Berechnung der Pauschale aus diesem Wert.

## aus `p9_6_erstausbildung_abgrenzung`

- **Satz 2: Eine Berufsausbildung als Erstausbildung nach Satz 1 liegt vor, wenn eine geordnete Ausbildung mit einer Mindestdauer von 12 Monaten bei vollzeitiger Ausbildung und mit einer Abschlussprüfung durchgeführt wird.**
  - Definiert lediglich, wann der Eingabewert 'erstausbildung_abgeschlossen' als erfüllt gilt, ohne die im Scope modellierte Abzugsregel selbst zu verändern.
- **Satz 3: Eine geordnete Ausbildung liegt vor, wenn sie auf der Grundlage von Rechts- oder Verwaltungsvorschriften oder internen Vorschriften eines Bildungsträgers durchgeführt wird.**
  - Unterdefinition zu Satz 2; betrifft nur die Bestimmung des Eingabewerts, nicht die Berechnung der abziehbaren Werbungskosten.
- **Satz 4: Ist eine Abschlussprüfung nach dem Ausbildungsplan nicht vorgesehen, gilt die Ausbildung mit der tatsächlichen planmäßigen Beendigung als abgeschlossen.**
  - Ergänzt die Definition des Abschlusses einer Erstausbildung; kein Einfluss auf die im Scope abgebildete Konditionallogik.
- **Satz 5: Eine Berufsausbildung als Erstausbildung hat auch abgeschlossen, wer die Abschlussprüfung einer durch Rechts- oder Verwaltungsvorschriften geregelten Berufsausbildung mit einer Mindestdauer von 12 Monaten bestanden hat, ohne dass er zuvor die entsprechende Berufsausbildung durchlaufen hat.**
  - Erweitert den Kreis der Fälle, in denen 'erstausbildung_abgeschlossen' wahr ist; die Scope-Logik bleibt davon unberührt.