# KI-geführter Dialog — Bedienkonzept

Julius, 2026-08-14, beim ersten eigenen Durchklicken durch die Oberfläche. Hier festgehalten,
weil es das Bedienkonzept ändert und nicht in der Chathistorie verschwinden soll.

## Das Ziel

Zwei gleichwertige Eingabewege auf **dieselben** Felder, jederzeit umschaltbar:

1. **Erzählen.** Der Nutzer schreibt frei, was auf ihn zutrifft. Das LLM übersetzt in Feldwerte.
2. **Klicken.** Der Nutzer beantwortet die nächste wichtige Frage im Fragebogen.

Kein Entweder-oder: „Wenn immer der Nutzer irgendwie neue Infos gibt, die das LLM direkt in
Fragebogenfelder ummünzen kann, dann sollte es das tun. Wenn er das nicht tut, dann kann er
vielleicht einfach weiterklicken und es kommen dann die next wichtigen Fragebogenfelder."

## Vier Punkte im Einzelnen

### 1. Einstieg über die KI

Statt sofort Fragebogen: die KI fragt zuerst nach den groben Eckdaten („wer bist du"). Der Nutzer
antwortet in einem Zug — Arbeitnehmer, Kapitalerträge, seit 27.4. arbeitslos.

### 2. Verstanden-Seite mit Bestätigung

Nach jeder Freitext-Eingabe zeigt das Produkt, was es verstanden hat, und fragt zurück:

> Das habe ich verstanden:
> • Arbeitnehmer
> • du hast Kapitalerträge
> • du bist seit 27.4. arbeitslos
> Soll ich das so eintragen?

Wichtig: **das Feld selbst wird angezeigt**, in das eingetragen würde — nicht nur ein Satz. Der
Nutzer bestätigt jeden Wert.

Das passt exakt auf die bestehende Sicherheitsarchitektur, ohne sie zu verändern: die KI schreibt
über `schreiber="llm:chat"` ausschließlich **vorläufige** Events (Store-Auflage A erzwingt
`zustand=vorlaeufig`, `signal_2=null`), und erst der menschliche Hold-Confirm setzt `signal_2`.
Die Verstanden-Seite ist die sichtbare Form dessen, was der Store ohnehin verlangt.

### 3. KI immer offen

Kein Modal, das man aufklappt und wieder schließt, sondern eine dauerhaft ansprechbare Hilfe —
„einfach als Hilfe", jederzeit etwas hinwerfen können.

### 4. „Erklär mir" erklärt wirklich

Heute öffnet der Knopf den Chat und der Nutzer muss selbst eine Frage stellen. Erwartet wird:
die KI **erklärt sofort** die aktuelle Frage. Der vorhandene Erklärtext ist dabei die erste
Antwort, danach sind Rückfragen möglich.

Die KI soll dabei berücksichtigen, was der Nutzer bereits geantwortet hat — die Erklärung eines
Feldes ist eine andere, wenn schon bekannt ist, dass er Arbeitnehmer ohne Kinder ist.

## Was davon schon steht

| Baustein | Zustand |
|---|---|
| LLM-Client + Chat-Handler | **fertig** (`api_llm._llm_vorschlaege`, `llm_client.complete`) |
| PII-Filter vor jedem Call | **fertig** — IdNr, IBAN, Datum, PLZ/Ort, Straße, Name werden maskiert |
| Audit je Call ohne Freitext | **fertig** |
| Vorschlag → vorläufiges Event | **fertig** (Store-Auflage A + Katalog-Check) |
| Hold-Confirm als zweites Signal | **fertig** |
| Konflikt-Erkennung (Feld schon gesetzt) | **fertig** (`konflikte`, `gross` für strukturelle) |
| Erklärung je Feld | **fertig**, aber nur im Rechenweg sichtbar: `/fall/<id>/feld/<fid>/warum` liefert Paragraph, Zitatanker, Herkunft |
| Fragereihenfolge nach Wichtigkeit | **fertig** seit 2026-08-14 (`traverser.gate_gewicht`) |

## Was fehlt

1. **LLM-Key.** Der einzige echte Blocker. Ohne Key/Base/Modell antwortet `/chat` mit 501 und dem
   Erklär-Vertrag — bewusst so, kein Mock-Call. **Julius-Cap: Freigabe + Kosten.**
2. **Kontext an das LLM.** `_llm_vorschlaege(freitext, katalog)` bekommt heute nur den
   Feld-Katalog, nicht die bereits beantworteten Werte. Für Punkt 1, 2 und 4 nötig.
3. **Verstanden-Seite.** Die Vorschläge erscheinen heute einzeln im Fluss mit ✦-Abzeichen; es
   fehlt die Sammelansicht mit Bestätigung.
4. **Layout „KI immer offen".** Heute `#chat-overlay` als Modal.
5. **„Erklär mir" umhängen** auf `/warum` + LLM-Erläuterung statt leerem Chat.

## Reihenfolge-Vorschlag

Punkt 2 (Kontext) lässt sich **ohne Key** bauen und testen — er vergrößert nur den Prompt. Punkt 5
teilweise ebenso: den vorhandenen `/warum`-Text als erste Antwort zeigen geht ohne LLM, die
Rückfragen brauchen ihn.

Alles andere hängt am Key. Deshalb: erst 2 und die Layout-Arbeit (3, 4), dann Cap-Entscheidung,
dann die LLM-abhängigen Teile scharf schalten.
