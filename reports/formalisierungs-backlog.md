# Formalisierungs-Backlog (aus scope_gaps)

Automatisch erzeugt aus den Judge-Verdikten der Gate-Kaskade. Enthalten sind ausschliesslich Norm-Teile der Klasse `unabhaengig`: sie liegen ausserhalb der formalisierten Signatur und aendern deren Ergebnis nicht. Norm-Teile, die in den Signatur-Scope hineinwirken, stehen NICHT hier - sie eskalieren im `scope_gap`-Gate, weil der Ausschnitt ohne sie falsch waere.

29 Item(s) aus 7 Regel(n).


## aus `p10_1_7_berufsausbildung`

- **Satz 2: Bei Ehegatten, die die Voraussetzungen des § 26 Absatz 1 Satz 1 erfüllen, gilt Satz 1 für jeden Ehegatten.**
  - Regelt die Anwendung des Höchstbetrags bei Zusammenveranlagung, liegt außerhalb des Scopes, der nur eine Person betrachtet, und ändert die Kappungslogik für eine Person nicht.
- **Satz 3: Zu den Aufwendungen im Sinne des Satzes 1 gehören auch Aufwendungen für eine auswärtige Unterbringung.**
  - Erweitert die Definition der Aufwendungen, die als Input in den Scope eingehen; die Kappungsberechnung selbst bleibt unverändert.
- **Satz 4: § 4 Absatz 5 Satz 1 Nummer 6b und 6c sowie § 9 Absatz 1 Satz 3 Nummer 4 und 5, Absatz 2, 4 Satz 8 und Absatz 4a sind bei der Ermittlung der Aufwendungen anzuwenden.**
  - Betrifft die Ermittlung der Aufwendungen (Input), nicht die Höchstbetragsberechnung; der Scope bleibt korrekt.

## aus `p24b_entlastungsbetrag`

- **Allein stehend im Sinne des Absatzes 1 sind Steuerpflichtige, die nicht die Voraussetzungen für die Anwendung des Splitting-Verfahrens (§ 26 Absatz 1) erfüllen oder verwitwet sind und keine Haushaltsgemeinschaft mit einer anderen volljährigen Person bilden, es sei denn, für diese steht ihnen ein Freibetrag nach § 32 Absatz 6 oder Kindergeld zu.**
  - Die Definition beeinflusst nur die Bestimmung des Eingabewerts 'alleinstehend', nicht die Berechnung des Entlastungsbetrags selbst.
- **wenn zu ihrem Haushalt mindestens ein Kind gehört, für das ihnen ein Freibetrag nach § 32 Absatz 6 oder Kindergeld zusteht**
  - Die Kriterien für die Zählung der Kinder sind außerhalb der Berechnung; der formalisierte Code verwendet nur die bereits ermittelte Anzahl.
- **Die Zugehörigkeit zum Haushalt ist anzunehmen, wenn das Kind in der Wohnung des allein stehenden Steuerpflichtigen gemeldet ist.**
  - Diese Vermutungsregel betrifft die Feststellung der Haushaltszugehörigkeit und damit den Eingabewert, nicht die Berechnung.
- **von der Summe der Einkünfte abziehen**
  - Die Art der steuerlichen Berücksichtigung (Abzug von der Summe der Einkünfte) liegt außerhalb des berechneten Geldbetrags.

## aus `p33_3_zumutbare_belastung`

- **Als Kinder des Steuerpflichtigen zählen die, für die er Anspruch auf einen Freibetrag nach § 32 Absatz 6 oder auf Kindergeld hat.**
  - The formalisation takes the number of children as input; this definition determines that input but does not alter the calculation logic inside the scope.
- **bei denen die Einkommensteuer ... nach § 32a Absatz 5 oder 6 (Splitting-Verfahren) zu berechnen ist**
  - The formalisation takes a boolean splitting as input; the determination of that boolean is outside the scope and does not affect the calculation logic inside.

## aus `p35a_2_3_haushaltsnahe`

- **§ 35a Abs. 1: „handelt es sich um eine geringfügige Beschäftigung im Sinne des § 8a des Vierten Buches Sozialgesetzbuch“**
  - Die Qualifikation als Minijob liegt außerhalb der Signatur; die Berechnung der Steuerermäßigung aus den bereits qualifizierten Aufwendungen bleibt unverändert.
- **§ 35a Abs. 2 Satz 1: „andere als in Absatz 1 aufgeführte haushaltsnahe Beschäftigungsverhältnisse oder für die Inanspruchnahme von haushaltsnahen Dienstleistungen, die nicht Dienstleistungen nach Absatz 3 sind“**
  - Die Abgrenzung der haushaltsnahen Dienstleistungen ist nicht Teil der Signatur; die Berechnung wird dadurch nicht geändert.
- **§ 35a Abs. 2 Satz 2: „für die Inanspruchnahme von Pflege- und Betreuungsleistungen sowie für Aufwendungen, die einem Steuerpflichtigen wegen der Unterbringung in einem Heim oder zur dauernden Pflege erwachsen“**
  - Pflege- und Heimkosten sind vom Scope ausgeschlossen; die Signatur modelliert nur die übrigen haushaltsnahen Dienstleistungen.
- **§ 35a Abs. 3 Satz 1: „Handwerkerleistungen für Renovierungs-, Erhaltungs- und Modernisierungsmaßnahmen“**
  - Die Eingrenzung auf bestimmte Handwerkerleistungen liegt außerhalb der Signatur; die Berechnung der Ermäßigung aus den bereits qualifizierten Arbeitskosten bleibt gleich.
- **§ 35a Abs. 3 Satz 2: „Dies gilt nicht für öffentlich geförderte Maßnahmen, für die zinsverbilligte Darlehen oder steuerfreie Zuschüsse in Anspruch genommen werden.“**
  - Der Ausschluss öffentlich geförderter Maßnahmen ist eine Vorbedingung für die Eingabe; die Berechnung selbst wird nicht modifiziert.
- **§ 35a Abs. 4 Satz 1: „in einem in der Europäischen Union oder dem Europäischen Wirtschaftsraum liegenden Haushalt“**
  - Die räumliche Voraussetzung betrifft die Anspruchsberechtigung, nicht die Berechnung der Höhe der Ermäßigung.
- **§ 35a Abs. 5 Satz 1: „soweit die Aufwendungen nicht Betriebsausgaben oder Werbungskosten darstellen und soweit sie nicht als Sonderausgaben oder außergewöhnliche Belastungen berücksichtigt worden sind“**
  - Das Verbot der Doppelberücksichtigung ist eine außerhalb der Signatur liegende Vorbedingung; die Ermittlung des Ermäßigungsbetrags bleibt unberührt.
- **§ 35a Abs. 5 Satz 2: „Der Abzug von der tariflichen Einkommensteuer nach den Absätzen 2 und 3 gilt nur für Arbeitskosten.“**
  - Die Beschränkung auf Arbeitskosten definiert die zulässige Eingabe; die Signatur setzt bereits voraus, dass nur Arbeitskosten übergeben werden.
- **§ 35a Abs. 5 Satz 3: „dass der Steuerpflichtige für die Aufwendungen eine Rechnung erhalten hat und die Zahlung auf das Konto des Erbringers der Leistung erfolgt ist“**
  - Rechnung und unbare Zahlung sind Anspruchsvoraussetzungen, die die Berechnung der Ermäßigung nicht verändern.
- **§ 35a Abs. 5 Satz 4: „Leben zwei Alleinstehende in einem Haushalt zusammen, können sie die Höchstbeträge nach den Absätzen 1 bis 3 insgesamt jeweils nur einmal in Anspruch nehmen.“**
  - Die Regelung betrifft die Aufteilung der Höchstbeträge zwischen zwei Personen; die Signatur berechnet die Ermäßigung für einen Haushalt ohne diese Aufteilung.
- **§ 35a Abs. 1, 2, 3: „auf Antrag“**
  - Das Antragserfordernis ist eine verfahrensrechtliche Bedingung, die außerhalb der Berechnungslogik liegt.

## aus `p9_1_3_nr5_doppelte_haushaltsfuehrung`

- **notwendige Mehraufwendungen, die einem Arbeitnehmer wegen einer beruflich veranlassten doppelten Haushaltsführung entstehen.**
  - Diese Voraussetzung betrifft den Grund der Abziehbarkeit, ändert aber nicht die Berechnung der Unterkunftskosten innerhalb des Scopes.
- **Eine doppelte Haushaltsführung liegt nur vor, wenn der Arbeitnehmer außerhalb des Ortes seiner ersten Tätigkeitsstätte einen eigenen Hausstand unterhält und auch am Ort der ersten Tätigkeitsstätte wohnt.**
  - Definiert den Tatbestand der doppelten Haushaltsführung, ohne die Höhe der abziehbaren Unterkunftskosten zu verändern.
- **Das Vorliegen eines eigenen Hausstandes setzt das Innehaben einer Wohnung sowie eine finanzielle Beteiligung an den Kosten der Lebensführung voraus.**
  - Präzisiert eine Tatbestandsvoraussetzung, die außerhalb des Scopes liegt und die Berechnung nicht beeinflusst.
- **Aufwendungen für die Wege vom Ort der ersten Tätigkeitsstätte zum Ort des eigenen Hausstandes und zurück (Familienheimfahrt) können jeweils nur für eine Familienheimfahrt wöchentlich abgezogen werden.**
  - Regelt eine andere Art von Aufwendungen (Familienheimfahrten), die nicht Gegenstand des Scopes sind.
- **Zur Abgeltung der Aufwendungen für eine Familienheimfahrt ist eine Entfernungspauschale von 0,38 Euro für jeden vollen Kilometer der Entfernung zwischen dem Ort des eigenen Hausstandes und dem Ort der ersten Tätigkeitsstätte anzusetzen.**
  - Betrifft die Berechnung der Familienheimfahrten, die außerhalb des Scopes liegt.
- **Nummer 4 Satz 3 bis 5 ist entsprechend anzuwenden.**
  - Verweist auf Regelungen zu anderen Werbungskosten, die für den Scope der Unterkunftskosten keine Rolle spielen.
- **Aufwendungen für Familienheimfahrten mit einem dem Steuerpflichtigen im Rahmen einer Einkunftsart überlassenen Kraftfahrzeug werden nicht berücksichtigt.**
  - Schließt bestimmte Aufwendungen bei Familienheimfahrten aus, hat aber keinen Einfluss auf die Unterkunftskosten.

## aus `p9_4a_verpflegungsmehraufwand`

- **Hat der Arbeitnehmer keine erste Tätigkeitsstätte, gelten die Sätze 2 und 3 entsprechend.**
  - Die Regel erweitert nur den Anwendungsbereich, ändert aber nicht die Berechnung der Pauschale selbst.

## aus `p9_6_erstausbildung_abgrenzung`

- **Sätze 2 bis 5, die definieren, wann eine Berufsausbildung als Erstausbildung vorliegt und wann sie als abgeschlossen gilt.**
  - Die Formalisierung verwendet den booleschen Input 'erstausbildung_abgeschlossen' als gegeben; die Definitionen ändern die konditionale Regel innerhalb des Scopes nicht.