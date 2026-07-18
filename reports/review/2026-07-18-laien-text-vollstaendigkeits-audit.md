# Laien-Text-Vollständigkeits-Pass — Audit (dev-2, UI-Vorbereitung)

**Auftrag:** Read-only-Audit aller askable-Felder auf fragetext_laie / hilfe_kurz / beispielwert
(Jargon? hilfreich? plausibel? Ton-konsistent?). **Nur VORSCHLÄGE — keine Änderung ohne OK.** LLM-frei.

## KOPFZEILE: Vollständig + überwiegend stark
- **92/92 askable-Felder haben alle 3 Laien-Elemente** (fragetext_laie + hilfe_kurz + beispielwert) — 0 Lücken.
- Ton durchgehend „du"-Form, konkret, laien-freundlich, mit Fundstellen-Hinweisen (Lohnsteuerbescheinigung-
  Zeilen, Steuerbescheid). Sehr gutes Fundament. Die folgenden Punkte sind Politur, keine Löcher.

## BEFUNDE (nach Priorität)

### 1. IMPLAUSIBLER beispielwert — vor_an_anteil_rv / vor_ag_anteil_rv (HOCH)
Beide beispielwert **3500000 Cent = 35.000 €**. Das ist als Renten­versicherungs-Anteil UNMÖGLICH: der
AN-Anteil ist auf ~8.984 € gedeckelt (Beitragsbemessungsgrenze 2025 × 9,3 %). Bei einem Bruttolohn-Beispiel
von 40.000 € läge der RV-AN-Anteil bei ~3.720 € = **372000 Cent**. Der aktuelle Wert ist ~10× zu hoch —
in der UI ein irreführender Prefill.
→ VORSCHLAG: vor_an_anteil_rv → 372000, vor_ag_anteil_rv → 372000 (bzw. konsistent zum Bruttolohn-Beispiel).
(Prüfen ob Test-Fixtures den Wert hart nutzen — est_mapping-Tests nutzen eigene Konstanten, nicht beispielwert.)

### 2. Vordruck-JARGON in hilfe_kurz — kap_gewinn_aktien (MITTEL)
hilfe: „Im Vordruck als Teilmenge der gesamten Kapitalerträge deklariert (die in der Summen-Zeile
enthaltenen Aktiengewinne)." — „Teilmenge"/„Summen-Zeile" ist Formular-Jargon, für den Laien nutzlos.
→ VORSCHLAG: „Nur die Gewinne aus verkauften Aktien (ein Teil deiner gesamten Kapitalerträge). Wichtig,
weil Aktienverluste nur mit Aktiengewinnen verrechnet werden dürfen."

### 3. DOPPELERFASSUNGS-Risiko — vv_werbungskosten vs. Einzelfelder (MITTEL, UX-Flow-Frage)
vv_werbungskosten fragt nach der **Summe** aller V+V-Kosten („Abschreibung, Zinsen, Reparaturen, Sonstiges
zusammen"), obwohl vv_gebaeude_afa / vv_schuldzinsen / vv_erhaltungsaufwand / vv_sonstige_wk dieselben
Posten EINZELN askable abfragen. Ein Laie, der beide sieht, trägt doppelt ein oder ist verwirrt.
→ VORSCHLAG (UI-Flow, deine Entscheidung): entweder vv_werbungskosten NICHT askable stellen (aus den
Einzelfeldern summieren) ODER die Einzelfelder ausblenden, wenn die Summe direkt eingegeben wird. Nicht
beide gleichzeitig zeigen.

### 4. POLARITÄTS-Frage — die 4 kein_-Flags (MITTEL, UI-Inversions-Bestätigung)
kein_gewinn/kein_kap/kein_vuv/kein_sonstige haben POSITIVE fragetexte („Hattest du Einkünfte aus X?") +
beispielwert **True**. Das Feld ist aber die ABWESENHEITS-Behauptung (kein_X=True = „hatte KEIN X"), die
Haut invertiert. Prefill-Risiko: zeigt die UI „Hattest du X? [Ja]", meint der Feldwert True aber „kein X"
→ falscher Vorbelegungs-Sinn.
→ FRAGE an dich: invertiert die Haut den beispielwert konsistent zum positiven fragetext? Falls nein →
beispielwert der 4 Flags auf False setzen (positiver fragetext „hatte X" = Normalfall Nein bei reinem AN
… bzw. was der Default sein soll). Bitte Flow bestätigen, dann ziehe ich nach.

### 5. Platzhalter-beispielwert — person_b_idnr (NIEDRIG)
beispielwert „00000000000" (11 Nullen) = offensichtlicher Platzhalter, nicht illustrativ.
→ VORSCHLAG: eine gut-geformte Beispiel-Nummer, z. B. „12345678901" (Format-Illustration, kein echter Wert).

### 6. Dichte hilfe_kurz — rentner_renten_art (NIEDRIG)
hilfe nennt „Versorgungswerk", „Rürup-Basisrente", „Ertragsanteil" — für den Ziel-Laien (Rentner) dicht.
Inhaltlich korrekt + erklärt; nur falls du maximale Zugänglichkeit willst, in zwei Sätze entzerren.
→ OPTIONAL, kein Muss.

## Nicht-Befunde (geprüft, OK)
Alle anderen ~85 Felder: fragetext klar + konkret, hilfe hilfreich (nicht bloß Norm-Zitat), beispielwert
plausibel, Ton konsistent. Besonders sauber: Verpflegung-Tage (28/14-€-Nennung), §35a-Handwerker (Arbeits-
kosten-vs-Material-Trennung + Unbar-Hinweis), §16-Betriebsart (Anlage-G/S/L-Erklärung), §33b (Merkzeichen-
Klartext), dhf-Doppelhaushalt (Zweitwohnung-Sprache).

## Empfehlung
Kern-Fix ist #1 (implausibler RV-beispielwert, HOCH) + #2 (kap-Jargon). #3/#4 sind UX-Flow-Entscheidungen
(brauchen dein Ruling zur UI-Logik, nicht nur Text). #5/#6 kosmetisch. Auf dein OK ziehe ich #1/#2/#5 (reine
Text/Wert-Fixes, meine Zone) nach; #3/#4 nach deinem Flow-Ruling. KEINE Änderung ohne dein Wort (Laien-Text
produktkritisch).
