"""LLM-Integration für Chat-Berater + Kontoauszug-Klassifikation (Bright-Line Isolation).

DER CHAT-BERATER LÄUFT IN DREI STUFEN (`_llm_dialog`), seit dem 2026-08-21:
  1. AUSSAGEN — der Nutzertext wird in einzelne Tatsachen zerlegt. Kein Katalog, ~1.000 Zeichen.
  2. THEMEN   — jede Aussage bekommt Regel-Kennungen. 62 Regeln à eine Zeile, ~7.000 Zeichen.
  3. WERTE    — nur noch die Felder der getroffenen Regeln liefern Vorschläge und Rückfragen.
Vorher war es ein Aufruf mit allen 321 Feldern: 96.679 Zeichen System-Prompt, davon 96 % Feldliste,
gegen 231 Zeichen Nutzertext. Der Umbau ist keine Sparmassnahme, sondern eine Genauigkeits-Frage —
gemessen kamen von fünf Tatsachen stabil drei an, und die zwei fehlenden fielen ohne jede Spur weg.
Deshalb trägt der Rückgabewert jetzt auch `aussagen` mit einem Status je Aussage: eine Angabe, für
die sich kein Feld fand, ist ein ERGEBNIS, kein stiller Verlust.

Dieses Modul ISOLIERT den llm_client-Import auf eine Datei (hier). api.py + Server
importieren diese Funktionen, ohne selbst llm_client zu kennen. Lazy imports innerhalb
der Funktionen halten die Abhängigkeit locker. Exportiert LlmNichtVerfuegbar damit
api.py sie in except-Clauses nutzen kann."""

import json
import re

import audit  # noqa: E402 — P1.6 Audit-Log (sys.path via api.py)
import flow  # noqa: E402 — Fluss-Mitschnitt, nur mit TAXGRAPH_FLOW=1
import kontoauszug_writer as KW
import traverser as TR  # noqa: E402 — nur lade_instanz_gruppen (Zählfeld je Instanz-Gruppe)
from pii_filter import filtere  # noqa: E402 — PII-Filter vor ausgehendem LLM-Call

# Exception für Exception-Handling in api.py (ohne dass api.py selbst llm_client importiert)
try:
    import llm_client
    LlmNichtVerfuegbar = llm_client.LlmNichtVerfuegbar
except ImportError:
    # Fallback wenn llm_client nicht da (dev-Umgebung)
    class LlmNichtVerfuegbar(Exception):
        pass


def _dialog_prompt(freitext: str, katalog: list[dict], kontext: str = "",
                   aussagen: list[dict] | tuple = ()) -> list[dict]:
    """STUFE 3 von dreien: aus einem Nutzersatz Feld-Werte VORSCHLAGEN **und** eine Frage
    BEANTWORTEN — jetzt aber nur noch über die Felder der Regeln, die Stufe 2 getroffen hat, und
    mit den Aussagen aus Stufe 1 als Gliederung daneben.

    `aussagen` ist die Liste aus Stufe 1 ({text, beleg}); sie steht als nummerierte Liste im
    Prompt, damit jeder Vorschlag und jede Rückfrage auf eine Nummer zeigen kann. Leer heißt: die
    Zerlegung hat nichts gefunden (reine Frage) — dann bleibt es beim Freitext allein, wie bisher.
    Der `freitext` bleibt die Nutzernachricht: die BELEGE müssen aus IHM stammen, nicht aus den
    Aussagen. Eine von Stufe 1 formulierte Aussage ist selbst schon Modellausgabe; ein Beleg, der
    gegen sie geprüft würde, belegte eine Modellausgabe mit einer anderen.

    Warum beides zusammen (Julius 2026-08-14): „‚Ein Satz an die KI' kann aber auch einfach eine
    Nachfrage sein." Zwei Knöpfe zwangen den Nutzer, seinen eigenen Satz vorher einzusortieren —
    und ein Satz ist oft beides („Ich fahre 15 km — zählt Homeoffice eigentlich als Arbeitstag?").
    Eine Vorab-Klassifikation im Code wäre nur dieselbe Zwangswahl, bloß unsichtbar und mit einer
    Fehlerquelle mehr. Also entscheidet nichts: das Modell füllt aus, was der Text hergibt, und
    antwortet, wenn gefragt wurde.

    System-Regel bleibt: die KI darf AUSSCHLIESSLICH die Felder aus dem übergebenen Katalog
    vorschlagen (askable + vorschlagbar; der Store-Katalog-Check ist die zweite Verteidigung), NUR
    als Vorschlag. `kontext` trägt das gerade offene Feld, seinen Zitatanker und die schon
    bestätigten Angaben — er ist für die ANTWORT da, nicht für die Vorschläge. Task-Wrapper
    (Handler-Schicht) — der Client (llm_client) kennt diesen Prompt nicht."""
    felder = "\n".join(
        f"- {f['feld_id']}: {f.get('fragetext_laie', '')}"
        f" (Typ {f.get('typ', '')}"
        + (f", Bereich {f['bereich']}" if f.get("bereich") else "")
        + (f", Werte {f['enum_werte']}" if f.get("enum_werte") else "")
        + ")"
        # Die Kurzhilfe sagt, WAS zum Feld gehört und wo es steht ("Steht auf der
        # Lohnsteuerbescheinigung Nr. 3", "Nach Abzug von Erstattungen"). Ohne sie ordnet die KI
        # nach dem Feldnamen zu und merkt nicht, wenn eine Angabe unvollständig ist.
        + (f"\n    dazu gehört: {f['hilfe_kurz']}" if f.get("hilfe_kurz") else "")
        for f in katalog)
    system = (
        "Du bist ein Steuer-Assistent mit ZWEI Aufgaben bei jeder Nachricht. Der Nutzer schreibt frei; "
        "sein Text kann Angaben enthalten, eine Frage sein oder BEIDES.\n"
        "AUFGABE 1 — Feld-Werte VORSCHLAGEN, für alles, was der Text an Angaben hergibt.\n"
        "AUFGABE 2 — seine Frage BEANTWORTEN, falls er eine gestellt hat.\n"
        "Beide Aufgaben gelten immer; du entscheidest nicht, welche der Nutzer gemeint hat, sondern "
        "erledigst jede, für die der Text etwas hergibt.\n"
        "Du SETZT nie einen Wert und triffst keine rechtliche Entscheidung — der Mensch bestätigt jeden Vorschlag. "
        "Du darfst NUR diese Felder vorschlagen (keine anderen):\n" + felder + "\n\n"
        "Geld-Beträge MUSST du als GANZZAHL in CENT angeben (EUR × 100), z.B. 2156,50 € → 215650. "
        "Niemals als EUR-Kommazahl oder EUR-Ganzzahl.\n"
        # Gemessen 2026-08-14, erster echter Lauf: aus "Ich bin Arbeitnehmer, verheiratet, fahre an
        # 220 Tagen 15 km zur Arbeit und habe 62000 Euro brutto verdient" schlug das Modell NEBEN
        # den vier echten Werten auch kein_gewinn/kein_kap/kein_vuv/kein_sonstige = true vor. Der
        # Nutzer hat das nicht gesagt — ein Arbeitnehmer kann sehr wohl ein Depot haben.
        # Eine erfundene ABWESENHEIT ist gefährlicher als ein erfundener Betrag: ein zu hoher
        # Betrag fällt beim Bestätigen auf, ein "nein, hatte ich nicht" klingt plausibel und wird
        # durchgewunken — und dann fehlt eine ganze Einkunftsart in der Erklärung (Under-
        # Deklaration). Deshalb diese Regel; die Felder bleiben vorschlagbar, wenn der Nutzer die
        # Abwesenheit wirklich ausspricht.
        # Erster Versuch endete mit "Im Zweifel dieses Feld weglassen" — das Modell bezog die
        # Zurückhaltung auf ALLE Felder und lieferte statt acht nur noch einen Vorschlag. Die Regel
        # muss also ausdrücklich sagen, dass sie nur für die kein_-Felder gilt.
        "SONDERREGEL, GILT NUR FÜR FELDER MIT PRÄFIX 'kein_': diese behaupten das FEHLEN einer "
        "Einkunftsart. Schlage sie NUR vor, wenn der Nutzer die Abwesenheit ausdrücklich nennt "
        "('ich habe keine Kapitalerträge', 'ich vermiete nichts'). Aus einer Berufsangabe wie 'ich "
        "bin Arbeitnehmer' folgt NICHT, dass es keine Kapitalerträge, Vermietung oder sonstigen "
        "Einkünfte gibt — ein Arbeitnehmer kann ein Depot haben. Im Zweifel NUR das betroffene "
        "kein_-Feld weglassen. Für alle anderen Felder gilt diese Zurückhaltung ausdrücklich NICHT: "
        "dort schlägst du jeden Wert vor, den die Beschreibung hergibt.\n"
        # OBJEKT, nicht Array — und das ist keine Geschmacksfrage: llm_client sendet
        # response_format={"type":"json_object"}, und dieser Modus verlangt ein Objekt an der
        # Wurzel. Der Prompt verlangte bis 2026-08-14 ein nacktes Array. Das Modell löste den
        # Widerspruch, indem es EIN Objekt lieferte — also genau EINEN Vorschlag, egal wie viele
        # Werte im Text standen. Gemessen am selben Satz: mal 8 Vorschläge (Array, gegen den
        # Modus), zweimal nur 1. Die Schwankung sah aus wie Modell-Laune und war ein
        # Format-Konflikt. _chat_parse versteht den Wrapper längst.
        # BELEGPFLICHT. Die Struktur erzwingt ohnehin das Schema (CHAT_SCHEMA, strict) — hier steht,
        # was ein guter Beleg IST. Das Zitat wird nach der Antwort gegen den Nutzertext geprüft;
        # was nicht wörtlich darin steht, wird verworfen. Ein Modell, das das weiß, rät weniger.
        "BELEGPFLICHT: zu jedem Vorschlag gehört ein `beleg` — ein WÖRTLICHES Zitat aus der "
        "Beschreibung des Nutzers, das genau diesen Wert trägt. Das Zitat muss Zeichen für Zeichen "
        "im Text vorkommen; erfinde oder paraphrasiere es nicht. Findest du keine Textstelle, lass "
        "das Feld weg — ein Vorschlag ohne Beleg wird verworfen.\n"
        # Neu mit dem zusammengelegten Kanal: wer fragen darf, fragt auch hypothetisch. „Was wäre,
        # wenn ich 62000 verdient hätte?" trägt eine Zahl UND einen Beleg — der Beleg-Filter greift
        # hier also NICHT, er prüft nur, ob das Zitat im Text steht. Die Grenze muss deshalb im
        # Prompt stehen. Zweite Verteidigung bleibt die Verstanden-Seite: dort steht das Zitat
        # neben dem Wert, und ein hypothetischer Satz fällt beim Lesen auf.
        "KEINE HYPOTHESEN ALS ANGABEN: eine Frage im Konjunktiv oder mit „wenn/angenommen/wäre' "
        "ist KEINE Angabe über den Nutzer. Aus „Was wäre, wenn ich 62000 verdient hätte?' folgt "
        "KEIN Vorschlag — das gehört in die Antwort, nicht in die Vorschläge.\n"
        # Gegengewicht zu den beiden Zurückhaltungs-Regeln darüber. Begründung ist die Bauart, NICHT
        # eine gemessene Wirkung: eine Vorsichtsregel färbt ab, wenn ihr keine Regel gegenübersteht,
        # die den Normalfall benennt — so drückte am 2026-08-14 ein „Im Zweifel weglassen" acht
        # Vorschläge auf einen. Mit der Zusammenlegung stehen jetzt ZWEI solche Regeln nebeneinander.
        #
        # EHRLICH GEMESSEN: der Anlass war, dass aus „verheiratet" kein veranlagung-Vorschlag mehr
        # kam. Diese Regel hat das NICHT behoben — über 6 Läufe desselben Satzes kam veranlagung
        # 1× (vorher 0× in 3 Läufen). Dabei zeigte sich der eigentliche Befund: das Modell antwortet
        # trotz temperature=0 NICHT deterministisch (derselbe Satz, derselbe Code: einmal 4, einmal
        # 3 Vorschläge). Einzelläufe taugen also nicht, um Prompt-Änderungen zu bewerten.
        # Die Regel bleibt, weil sie den Normalfall benennt und die beiden Schutzregeln nachweislich
        # nicht aufweicht (Hypothese und reine Frage liefern weiterhin 0 Vorschläge). Wer verlässlich
        # „verheiratet → Zusammenveranlagung" will, braucht eine Vorbelegung in unserem Code, kein
        # Prompt-Zureden.
        # RECHNEN STATT FRAGEN (Julius, 2026-08-23). Sein Verlauf zeigte den Fall: "vor juli 2025
        # habe ich 50k pro jahr verdient" plus "ich bin arbeitslos seit juli 2025" — daraus folgt
        # 50.000 ÷ 12 × 6. Die KI fragte stattdessen nach dem genauen Betrag, und das ist eine
        # Rückfrage zu viel: die Multiplikation steht im Satz.
        # Der `rechenweg` ist PFLICHTFELD im Schema (null, wenn nicht gerechnet wurde) und wird
        # von _rechenweg_geprueft() deterministisch NACHGERECHNET — das Modell darf rechnen, aber
        # nicht allein verantworten. Deshalb steht hier "zeig die Bestandteile", nicht "rechne".
        "AUSRECHNEN, WO DIE ANGABEN ES HERGEBEN: nennt der Nutzer einen Betrag je Zeitraum und "
        "einen Zeitraum, dann rechne — 'vor Juli 50.000 pro Jahr verdient' plus 'seit Juli "
        "arbeitslos' ergibt sechs von zwölf Monaten. Trag den ausgerechneten Wert ein und fülle "
        "`rechenweg` mit Basis, Faktor und einer Erklärung in einem Satz; der Nutzer sieht sie und "
        "kann korrigieren. Frag NICHT nach einer Zahl, die sich aus dem Gesagten ergibt. Wo nichts "
        "zu rechnen ist, bleibt `rechenweg` null. Bei Geld sind Basis und Wert in CENT.\n"
        "TATSACHEN ÜBERSETZT DU IMMER: was der Nutzer als Tatsache über sich sagt, wird zum "
        "Vorschlag — auch dann, wenn das Feld eine WAHL abbildet und der Text nur die übliche Wahl "
        "nahelegt. Vorgeschlagen ist nicht gesetzt: der Mensch sieht den Vorschlag neben deinem "
        "Zitat und bestätigt oder ändert ihn. Zurückhaltung gilt NUR für die beiden Fälle oben "
        "(kein_-Felder ohne ausgesprochene Abwesenheit, und Hypothesen).\n\n"
        # RÜCKFRAGEN STATT RATEN. Der Anlass ist gemessen: aus „bis Juni 100k p.a." wurde ein
        # Jahresbrutto von 100.000 €, mit `unsicher: false` daneben — 70.000 € zu viel. Das Modell
        # hatte kein Feld, in das der Zweifel gepasst hätte, also floss er in eine Zahl.
        # Die Verdrängung (Rückfrage ersetzt Vorschlag) steht hier UND wird unten deterministisch
        # durchgesetzt (`_rueckfrage_verdraengt`): stünde die Vermutung schon im Fall, während die
        # Frage noch offen ist, hätte der Nutzer sie beim Bestätigen längst durchgewunken.
        # ZWEITER ANLAUF, 2026-08-21. Die erste Fassung endete mit „Rückfragen sind für Lücken da,
        # nicht für Höflichkeit" — als vierte Vorsichtsregel neben den zwei bestehenden. Gemessen an
        # Julius' Text: das Modell schrieb daraufhin ZERO Rückfragen und wurde zugleich bei den
        # Vorschlägen stiller (2 statt 3 Tatsachen). Es nahm die Zurückhaltung mit und den neuen
        # Kanal nicht an — dieselbe Abfärbung, die der Kommentar 30 Zeilen weiter oben für den
        # 2026-08-14 festhält. Eine Vorsichtsregel braucht die Regel, die den Normalfall benennt.
        # Deshalb steht hier jetzt die PFLICHT zuerst und die Grenze danach.
        + "RÜCKFRAGEN STATT RATEN, UND STATT SCHWEIGEN: gibt der Text zu einem Feld das Thema her, "
        "aber nicht den Wert, dann MUSST du eine RÜCKFRAGE stellen. „Bis Juni 100.000 im Jahr' sagt "
        "NICHT, was in diesem Jahr zugeflossen ist — frag danach, statt die Jahreszahl zu setzen. "
        "„Ich hatte Ausgaben für meine Gesundheit' nennt das Feld, aber keinen Betrag — frag nach "
        "dem Betrag. Das ist der Normalfall für jede Angabe ohne Zahl, nicht die Ausnahme: eine "
        "genannte Tatsache einfach zu übergehen ist die EINZIGE Antwort, die hier falsch ist. "
        "Zwei Grenzen: eine Rückfrage ERSETZT den Vorschlag zu diesem Feld (nenne nie beides zum "
        "selben Feld), und ist ein Wert eindeutig, frag nicht, sondern schlag ihn vor.\n\n"
        + (_aussagen_block(aussagen) if aussagen else "")
        + (kontext + "\n\n" if kontext else "")
        + "Für die ANTWORT: schreib auf Deutsch, in der Du-Form, höchstens fünf Sätze, ohne "
        "Fachjargon (ein unvermeidbares Fachwort erklärst du im selben Satz). Stütze dich auf den "
        "oben zitierten Gesetzestext, falls einer angegeben ist. Nenne NIE einen konkreten Betrag "
        "als den Wert des Nutzers — er trägt jeden Wert selbst ein. Bist du dir nicht sicher, sag "
        "das ausdrücklich und setze \"unsicher\": true, statt zu raten. Hat der Nutzer gar nichts "
        "gefragt, ist `antwort` ein LEERER String — dann erfinde keine Belehrung.\n"
        "Antworte AUSSCHLIESSLICH mit einem JSON-OBJEKT der Form "
        "{\"vorschlaege\": [{\"feld_id\":\"…\",\"wert\":…,\"beleg\":\"Zitat\",\"begruendung\":\"kurz\","
        "\"aussage\":0}], "
        "\"rueckfragen\": [{\"frage\":\"…\",\"feld_id\":\"…\",\"aussage\":0}], "
        "\"antwort\":\"…\", \"unsicher\":false}. "
        "Die Liste enthält EINEN Eintrag JE FELD, für das die Beschreibung einen konkreten Wert "
        "hergibt — nenne alle, die du erkennst, nicht nur den ersten. Kein Treffer → "
        "\"vorschlaege\": []. Kein Fließtext außerhalb des JSON.")
    return [{"role": "system", "content": system}, {"role": "user", "content": freitext}]


def _aussagen_block(aussagen) -> str:
    """Die Aussagen aus Stufe 1 als nummerierte Liste für den Stufe-3-Prompt.

    Nummeriert, weil `vorschlaege[].aussage` und `rueckfragen[].aussage` auf diese Nummern zeigen —
    daran hängt, welche Aussage am Ende als verwertet gilt und welche als offen gemeldet wird. Ohne
    diese Zuordnung wäre eine untergegangene Aussage wieder unsichtbar, und genau das ist der Fehler,
    gegen den die drei Stufen gebaut sind."""
    zeilen = "\n".join(f"[{i}] {a.get('text', '')}" for i, a in enumerate(aussagen))
    return ("DAS HABEN WIR AUS SEINER NACHRICHT HERAUSGELESEN — jede Zeile ist eine Tatsache, die "
            "er über sich gesagt hat:\n" + zeilen + "\n"
            "ARBEITE DIESE LISTE ZEILE FÜR ZEILE AB. Zu JEDER Zeile gehört ein Eintrag: ein "
            "Vorschlag, wenn der Wert im Text steht — sonst eine Rückfrage. Übergehen darfst du "
            "eine Zeile nur, wenn die Feldliste oben zu ihrem Thema wirklich nichts enthält; "
            "Unsicherheit ist KEIN Grund zum Übergehen, dafür ist die Rückfrage da. Gib in "
            "`aussage` die Nummer der Zeile an, auf die sich der Eintrag stützt.\n"
            # GEMESSEN 2026-08-24: ohne diesen Absatz kamen auf fünf Aussagen 21 Rückfragen. Der
            # Grund steht eine Regel weiter oben — „gibt der Text zu einem Feld das Thema her,
            # frag nach" — und „2 Kinder" gibt jedem Kind-Feld sein Thema (Vorname, Geburtsdatum,
            # anderer Elternteil, Zeitraum). Das Modell hat wörtlich befolgt, was dastand.
            "HÖCHSTENS EINE RÜCKFRAGE JE ZEILE, und sie fragt nach GENAU EINER Sache. Eine Zeile "
            "lässt eine Unklarheit offen — die klärst du. Frag NICHT alle Felder ab, deren Thema "
            "die Zeile berührt: „zwei Kinder\" berührt Vornamen, Geburtsdaten und Elternteile, "
            "aber die stehen im Fragebogen und sind dort nicht verloren. Und bündle nie zwei "
            "Fragen in einen Satz („Wie heissen sie und wann sind sie geboren?\") — der Nutzer hat "
            "genau EIN Eingabefeld dafür und kann nur eine der beiden beantworten.\n\n")


# JSON-Schema, das der Provider ERZWINGT (OpenRouter structured_outputs, strict). Ersetzt die
# Bitte im Prompt durch eine Zusage des Anbieters — vorher lieferte das Modell im json_object-Modus
# mal ein Array, mal ein Einzelobjekt, und die Zahl der Vorschläge schwankte deshalb zwischen 8 und 1.
#
# `beleg` ist der Grund für dieses Schema (Julius 2026-08-14: "wir müssen das modell zwingen den
# beleg für die behauptung (als quote des users zb) mit zu schicken"). Es ist required und muss ein
# WÖRTLICHES Zitat aus der Nutzereingabe sein. Das ist mehr als Dokumentation: _beleg_geprueft()
# unten verwirft jeden Vorschlag, dessen Beleg nicht im Text steht — ein deterministischer Filter
# gegen erfundene Werte, der nicht davon abhängt, dass sich das Modell an eine Prompt-Regel hält.
#
# `antwort`/`unsicher` kamen 2026-08-14 dazu, als Vorschlags- und Erklär-Kanal zu EINEM Aufruf
# wurden. Beides required, damit die Struktur nicht davon abhängt, ob das Modell den Text für eine
# Frage hielt — leerer String heißt „keine Frage gestellt". `unsicher` zwingt zu einer
# ausdrücklichen Aussage darüber, ob die Antwort aus dem Gesetzestext folgt: wer ein Feld setzen
# MUSS, gibt Zweifel eher zu als jemand, der höflich darum gebeten wird.
DIALOG_SCHEMA = {
    "name": "dialog",
    "strict": True,
    "schema": {
        "type": "object",
        "properties": {
            "vorschlaege": {
                "type": "array",
                "description": "Ein Eintrag je Feld, für das die Beschreibung einen konkreten Wert hergibt.",
                "items": {
                    "type": "object",
                    "properties": {
                        "feld_id": {"type": "string",
                                    "description": "Exakt eine feld_id aus der vorgegebenen Liste."},
                        "wert": {"type": ["string", "number", "boolean"],
                                 "description": "Der vorgeschlagene Wert. Geld IMMER als ganzzahlige CENT."},
                        "beleg": {"type": "string",
                                  "description": "WÖRTLICHES Zitat aus der Nutzereingabe, das diesen Wert "
                                                 "belegt. Muss Zeichen für Zeichen im Text vorkommen. Kein "
                                                 "Beleg möglich → Feld weglassen."},
                        "begruendung": {"type": "string",
                                        "description": "Kurz: wie aus dem Zitat der Wert folgt."},
                        "aussage": {"type": "integer",
                                    "description": "Nummer der Aussage aus der Liste oben, auf die "
                                                   "sich dieser Wert stützt. Keine passende: -1."},
                        # RECHNEN STATT FRAGEN — Julius, 2026-08-23: "anteilig vom jahresbrutto ist
                        # aber einfach zu rechnen. genau so könnte das zu erwartende alg berechnet
                        # werden. und vorgeschlagen werden. das würde es dem nutzer einfacher machen."
                        # Anlass ist sein Verlauf: "50k pro jahr" + "seit juli arbeitslos" ergibt
                        # 6 × 50.000/12; die KI fragte stattdessen nach dem genauen Betrag.
                        #
                        # DAS MODELL RECHNET NICHT ALLEIN. Es liefert die BESTANDTEILE, und
                        # `_rechenweg_geprueft` rechnet deterministisch nach — passt es nicht,
                        # fliegt der Vorschlag raus. Dieselbe Bauart wie das Beleg-Gate: nicht darauf
                        # vertrauen, dass sich das Modell an eine Prompt-Regel hält, sondern die
                        # Behauptung selbst nachprüfen. Eine Rechnung, die das Modell allein
                        # verantwortet, hat in einer Steuersoftware nichts verloren.
                        #
                        # `required` und nullable statt optional: ein OPTIONALES Feld ist eines, das
                        # das Modell weglässt — dieselbe Lehre, die `rueckfragen` am 2026-08-21
                        # fünfzehn Läufe gekostet hat.
                        "rechenweg": {
                            "type": ["object", "null"],
                            "description": "NUR wenn der Wert aus einer Angabe des Nutzers "
                                           "AUSGERECHNET wurde (Jahresgehalt anteilig auf Monate, "
                                           "Monatsbetrag auf das Jahr). Sonst null.",
                            "properties": {
                                "basis": {"type": "number",
                                          "description": "Der genannte Ausgangsbetrag, in derselben "
                                                         "Einheit wie `wert` (Geld: CENT)."},
                                "faktor": {"type": "number",
                                           "description": "Womit die Basis multipliziert wurde: 0.5 "
                                                          "für 6 von 12 Monaten, 6 für sechs "
                                                          "Monatsbeträge."},
                                "erklaerung": {"type": "string",
                                               "description": "Der Rechenweg in einem Satz, für den "
                                                              "Nutzer lesbar: '50.000 € pro Jahr "
                                                              "÷ 12 × 6 Monate'."},
                            },
                            "required": ["basis", "faktor", "erklaerung"],
                            "additionalProperties": False,
                        },
                    },
                    "required": ["feld_id", "wert", "beleg", "begruendung", "aussage", "rechenweg"],
                    "additionalProperties": False,
                },
            },
            # DIE ZWEITE HÄLFTE VON STUFE 3, und der Grund für sie ist ein gemessener Fehler:
            # aus „bis Juni 100k p.a." schloss das Modell auf ein Jahresbrutto von 100.000 € und
            # setzte `unsicher: false` dazu — 70.000 € zu viel Einkommen, aufgefangen nur davon,
            # dass ein Mensch es beim Bestätigen las. Ein Modell, das nur `vorschlaege` füllen
            # kann, MUSS raten: es hat kein Feld, in das ein Zweifel passt. Dieses ist es.
            #
            # IN `required`, UND DAS IST DER GANZE UNTERSCHIED — gemessen, nicht überlegt.
            # Der erste Entwurf liess die Eigenschaft optional, damit ein Anbieter, der sie nicht
            # liefert, die Anfrage trotzdem beantworten kann. Ein echter Endpunkt akzeptiert das
            # auch (2026-08-21 geprüft). Nur: das Modell hat die Liste dann in FÜNFZEHN Läufen
            # ausnahmslos leer gelassen — quer durch drei Prompt-Fassungen, zwei Katalog-Grössen
            # und jede Zurede, die mir einfiel. Ein optionales Feld ist eines, das man weglässt.
            # Mit `required` daneben, gleicher Prompt, gleicher Text: 3 von 3 Läufen mit
            # Rückfragen — und zwar genau zu den beiden Angaben, die ein Thema nennen und keine
            # Zahl („Wie viel Arbeitslosengeld hast du dieses Jahr insgesamt bezogen?", „Wie hoch
            # waren deine Gesundheitsausgaben?"). Von fünf Tatsachen kamen 5/5 an statt 3/5.
            # Dieselbe Lehre wie bei `antwort`/`unsicher` eine Ebene tiefer: wer ein Feld füllen
            # MUSS, denkt über seinen Inhalt nach; wer darf, lässt es weg.
            # Ein Anbieter, der die Eigenschaft trotzdem nicht liefert, bricht nichts:
            # `_rueckfragen_parse` liest ein fehlendes Feld weiter als leere Liste.
            "rueckfragen": {
                "type": "array",
                "description": "Offene Punkte, bei denen der Text den Wert NICHT hergibt. Eine "
                               "Rückfrage ERSETZT den Vorschlag zu diesem Feld — schreibe niemals "
                               "beides zum selben Feld.",
                "items": {
                    "type": "object",
                    "properties": {
                        "frage": {"type": "string",
                                  "description": "Die Rückfrage an den Nutzer, Du-Form, ein Satz."},
                        "feld_id": {"type": "string",
                                    "description": "Das Feld, um das es geht — exakt eine feld_id "
                                                   "aus der Liste. Keines passend: leerer String."},
                        "aussage": {"type": "integer",
                                    "description": "Nummer der Aussage, aus der die Unklarheit "
                                                   "stammt. Keine passende: -1."},
                    },
                    "required": ["frage", "feld_id", "aussage"],
                    "additionalProperties": False,
                },
            },
            "antwort": {"type": "string",
                        "description": "Antwort auf die Frage des Nutzers: höchstens fünf Sätze, Du-Form, "
                                       "ohne Fachjargon. Hat er nichts gefragt: LEERER String."},
            "unsicher": {"type": "boolean",
                         "description": "true, wenn die Antwort NICHT sicher aus dem angegebenen "
                                        "Gesetzestext oder der Feldbeschreibung folgt. Ohne Antwort: false."},
        },
        "required": ["vorschlaege", "rueckfragen", "antwort", "unsicher"],
        "additionalProperties": False,
    },
}


def _normalisiert(s: str) -> str:
    """Kleinschreibung, Whitespace vereinheitlicht — für den Beleg-Vergleich. Ein Modell zitiert
    gern mit anderer Groß-/Kleinschreibung oder normalisierten Leerzeichen; daran soll ein
    inhaltlich richtiger Beleg nicht scheitern."""
    return " ".join((s or "").lower().split())


def _beleg_geprueft(vorschlaege: list[dict], freitext: str) -> tuple[list[dict], list[dict]]:
    """(behalten, verworfen) — ein Vorschlag überlebt nur mit Beleg, der WÖRTLICH im Freitext steht.

    Der eigentliche Schutz hinter dem Beleg-Zwang: das Modell kann eine Begründung erfinden, aber
    kein Zitat, das im Text nicht vorkommt. Dieser Abgleich ist deterministisch und hängt an keiner
    Prompt-Befolgung. Geprüft wird gegen den PII-GEFILTERTEN Text — genau den hat das Modell
    gesehen, alles andere würde legitime Belege verwerfen.

    KURZE BELEGE brauchen eine WORTGRENZE, keine Mindestlänge. Bis 2026-08-23 galt hier
    `len(beleg) >= 3`, begründet mit „'5' steht in fast jedem Text und belegt nichts". Der Grund
    stimmt, die Umsetzung war zu grob — und sie sabotierte ausgerechnet den Kanal, der einen Tag
    zuvor gebaut worden war:

        Rückfrage: "An wie vielen Tagen bist du dieses Jahr zur Arbeit gefahren?"
        Antwort:   "70"
        Ergebnis:  ohne_beleg — verworfen, weil zwei Zeichen

    Gemessen im echten Nutzerlauf. Die Antwort auf eine Rückfrage IST typischerweise eine kurze
    Zahl; eine Längenschwelle trifft dort systematisch das Richtige. Was die Regel eigentlich
    meinte, ist etwas anderes: eine Ziffer, die nur ZUFÄLLIG in einer längeren Zahl steckt, belegt
    nichts. Das ist eine Frage der Wortgrenze, nicht der Länge — „5" in „15000" ist kein Beleg,
    „5" als eigenes Wort schon.

    Ab drei Zeichen bleibt es beim Teilstring-Vergleich: dort ist ein zufälliges Vorkommen
    unwahrscheinlich genug, und das Modell zitiert gern Wortteile („20km" aus „20km mit dem auto"),
    die eine Wortgrenzen-Prüfung fälschlich verwürfe."""
    behalten, verworfen = [], []
    heuhaufen = _normalisiert(freitext)
    for v in vorschlaege:
        beleg = _normalisiert(v.get("beleg", ""))
        if beleg and (beleg in heuhaufen if len(beleg) >= 3
                      else re.search(rf"(?<!\w){re.escape(beleg)}(?!\w)", heuhaufen) is not None):
            behalten.append(v)
        else:
            verworfen.append(v)
    return behalten, verworfen


def _chat_parse(text: str) -> list[dict]:
    """Roher LLM-Text → Liste {feld_id, wert, begruendung}. Toleriert ein Objekt-Wrapper ({\"vorschlaege\":[…]})
    oder ein nacktes Array. Nicht-Liste/kaputtes JSON → [] (kein Vorschlag ist besser als ein Müll-Vorschlag)."""
    try:
        j = json.loads(text)
    except Exception:
        return []
    if isinstance(j, dict):
        for k in ("vorschlaege", "vorschläge", "suggestions", "felder"):
            if isinstance(j.get(k), list):
                j = j[k]
                break
        else:
            j = [j] if "feld_id" in j else []
    if not isinstance(j, list):
        return []
    out = []
    for v in j:
        if isinstance(v, dict) and "feld_id" in v and "wert" in v:
            out.append({"feld_id": str(v["feld_id"]), "wert": v["wert"],
                        # Das Zitat, auf das sich der Vorschlag stützt. Wird gegen den Freitext
                        # geprüft (_beleg_geprueft) und wandert bis in die Antwort, damit die
                        # Oberfläche neben jedem Wert zeigen kann, WORAUS er stammt.
                        "beleg": str(v.get("beleg", ""))[:300],
                        "begruendung": str(v.get("begruendung", ""))[:200],
                        # Rückverweis auf die Aussage aus Stufe 1. Fehlt er (altes Format, oder ein
                        # Anbieter, der die Eigenschaft wegliess), ist er None — der Vorschlag gilt
                        # dann, er ist nur keiner Aussage zuzurechnen.
                        "aussage": _index(v.get("aussage"))})
    return out


def _index(w) -> int | None:
    """Eine Aussage-Nummer aus der Modellantwort, oder None. `-1` ist der im Schema vorgesehene Weg
    zu sagen „keine passende Aussage"; alles Unbrauchbare landet ebenfalls bei None."""
    try:
        i = int(w)
    except (TypeError, ValueError):
        return None
    return i if i >= 0 else None


def _rueckfragen_parse(text: str, felder: int) -> list[dict]:
    """Roher LLM-Text → Liste {frage, feld_id, aussage}. Fehlendes Feld/kaputtes JSON → [].

    `felder` ist die Zahl der Felder, die Stufe 3 überhaupt sehen durfte. Ist sie 0, kann es keine
    feld-bezogene Rückfrage geben — dann ist eine trotzdem gelieferte Rückfrage eine Erfindung über
    ein Feld, das gar nicht zur Debatte stand, und fällt weg."""
    try:
        j = json.loads(text)
    except Exception:                                        # noqa: BLE001
        return []
    if not isinstance(j, dict) or not isinstance(j.get("rueckfragen"), list):
        return []
    out = []
    for r in j["rueckfragen"]:
        frage = str((r or {}).get("frage", "")).strip()[:300] if isinstance(r, dict) else ""
        if not frage:
            continue
        fid = str(r.get("feld_id", "") or "").strip()
        if fid and not felder:
            continue
        out.append({"frage": frage, "feld_id": fid, "aussage": _index(r.get("aussage"))})
    return out


# Absolute Obergrenze für EINE Runde. Die Regel darunter (eine je Aussage) ist die inhaltliche;
# diese hier fängt den Fall ab, dass Stufe 1 selbst viele Aussagen liefert. Mehr als acht Fragen
# hintereinander ist kein Dialog mehr, sondern ein Fragebogen mit Gesprächsanstrich — und den gibt
# es daneben bereits. Eine Einzelgrenze ohne Anzahlgrenze ist an anderer Stelle dieses Hauses schon
# dreimal aufgelaufen; deshalb steht hier beides.
RUECKFRAGEN_MAX = 8


def _rueckfragen_gebuendelt(rueckfragen: list[dict]) -> tuple[list[dict], int]:
    """Höchstens EINE Rückfrage je Aussage, höchstens RUECKFRAGEN_MAX insgesamt.

    GEMESSEN 2026-08-24, Live-Lauf: auf „ich bin verheiratet, habe 2 kinder, fuhre 20km …" kamen
    **21 Rückfragen** — darunter „Wie heissen die anderen Elternteile deiner Kinder?" und „Wann sind
    die anderen Elternteile deiner Kinder geboren?". Frühere Läufe mit demselben Text lagen bei 5–8.

    Das war keine Verirrung des Modells, sondern die Anweisung wörtlich befolgt: der Prompt sagt
    „gibt der Text zu einem Feld das Thema her, aber nicht den Wert, dann MUSST du eine RÜCKFRAGE
    stellen". „2 Kinder" gibt JEDEM Kind-Feld sein Thema — Vorname, Geburtsdatum, Zeitraum, anderer
    Elternteil. Also fragte es zu allen.

    Die Korrektur ist inhaltlich, nicht kosmetisch: eine Rückfrage klärt, was der Nutzer GESAGT hat.
    Zu einer Aussage gibt es genau eine Unklarheit, nämlich die, die sie offen lässt. Alles Weitere
    sind Fragebogen-Felder — und die stehen ohnehin im Fragebogen, unverloren.

    Deterministisch statt als Prompt-Bitte, aus demselben Grund wie das Beleg-Gate und
    `_rueckfrage_verdraengt`: eine Regel, die nur im Prompt steht, gilt, solange das Modell mag.
    Der Prompt sagt es zusätzlich — beides, nicht eines von beidem.

    Rückgabe: (behalten, zurueckgestellt). Die Zahl wandert bis in die Oberfläche; still kürzen
    hiesse, dem Nutzer eine Vollständigkeit vorzuspiegeln, die er nicht bekommen hat.
    """
    gesehen: set[int] = set()
    behalten: list[dict] = []
    for r in rueckfragen:
        # `aussage: -1` heisst „keine passende Aussage". Auch das ist eine Gruppe, sonst käme eine
        # Flut heimatloser Fragen ungefiltert durch und nur die absolute Grenze griffe.
        nr = r.get("aussage", -1)
        if nr in gesehen or len(behalten) >= RUECKFRAGEN_MAX:
            continue
        gesehen.add(nr)
        behalten.append(r)
    return behalten, len(rueckfragen) - len(behalten)


def _rueckfrage_verdraengt(vorschlaege: list[dict], rueckfragen: list[dict]) -> list[dict]:
    """Auflage: eine Rückfrage ERSETZT den Vorschlag zum selben Feld, sie begleitet ihn nicht.

    Deterministisch und nicht als Prompt-Bitte, aus demselben Grund wie das Beleg-Gate: die Regel
    darf nicht davon abhängen, dass sich das Modell an sie hält. Stünde die Vermutung als
    vorläufiges Event im Fall, während die Frage daneben noch offen ist, hätte der Nutzer sie beim
    Bestätigen längst durchgewunken — und die Rückfrage käme zu spät für den Wert, den sie klären
    sollte."""
    gefragt = {r["feld_id"] for r in rueckfragen if r.get("feld_id")}
    return [v for v in vorschlaege if v.get("feld_id") not in gefragt]


def _antwort_parse(text: str) -> tuple[str, bool]:
    """(antwort, unsicher) aus der Dialog-Antwort. Kaputtes JSON oder fehlende Felder → ("", False):
    dann bleibt die Antwort aus, statt dass eine halbe Zeichenkette als Erklärung durchgeht — und
    die Vorschläge (die _chat_parse eigenständig liest) hängen nicht daran."""
    try:
        j = json.loads(text)
    except Exception:                                        # noqa: BLE001
        return "", False
    if not isinstance(j, dict):
        return "", False
    return str(j.get("antwort") or "").strip()[:2000], bool(j.get("unsicher"))


# ============================================================== STUFE 1: was hat er überhaupt gesagt
#
# WARUM ES DIESE STUFE GIBT, in einer Zahl: der alte EINE Aufruf trug 96.679 Zeichen System-Prompt,
# davon 93.804 Feldliste — 96 %. Julius' Nachricht daneben: 231 Zeichen. Verhältnis 406 : 1. Das
# Modell sollte in 321 Feldbeschreibungen die zwei bis fünf finden, die passen, und tat es
# unzuverlässig: an genau diesem Text kamen über vier Läufe stabil 3 von 5 Tatsachen an, „seit Juli
# arbeitslos" und „Ausgaben für die Gesundheit" NIE.
#
# Diese Stufe bekommt gar keinen Katalog. Sie tut das, worin ein Sprachmodell gut ist — einen Satz in
# seine Behauptungen zerlegen — und nichts sonst. Was danach kommt, arbeitet auf einer Liste statt auf
# einem Absatz, und eine Liste kann man abarbeiten und nachzählen.
AUSSAGEN_SCHEMA = {
    "name": "aussagen",
    "strict": True,
    "schema": {
        "type": "object",
        "properties": {
            "aussagen": {
                "type": "array",
                "description": "Eine Tatsachenaussage je Eintrag, in der Reihenfolge des Textes.",
                "items": {
                    "type": "object",
                    "properties": {
                        "text": {"type": "string",
                                 "description": "Die Tatsache in einem kurzen Satz, dritte Person "
                                                "('Der Nutzer ist ledig')."},
                        "beleg": {"type": "string",
                                  "description": "WÖRTLICHES Zitat aus der Nachricht, Zeichen für "
                                                 "Zeichen, das genau diese Aussage trägt."},
                    },
                    "required": ["text", "beleg"],
                    "additionalProperties": False,
                },
            },
        },
        "required": ["aussagen"],
        "additionalProperties": False,
    },
}


def _aussagen_prompt(freitext: str) -> list[dict]:
    """Stufe 1: Nachricht → einzelne Tatsachenaussagen. KEIN Katalog, unter 1.000 Zeichen.

    Die drei Verbote unten sind dieselben wie in Stufe 3 und stehen aus demselben Grund hier: was in
    Stufe 1 als Tatsache durchgeht, wandert unbesehen in die beiden Folgestufen. Eine erfundene
    Abwesenheit („er hat keine Kapitalerträge") wäre hier sogar gefährlicher als dort — sie käme in
    Stufe 3 nicht mehr als Vermutung an, sondern als Feststellung von uns selbst."""
    system = (
        "Zerlege die Nachricht des Nutzers in EINZELNE Tatsachenaussagen über ihn. Eine Aussage = "
        "eine Tatsache: 'Der Nutzer ist ledig', 'Der Nutzer ist seit Juli arbeitslos'. Trenne, was "
        "er in einem Satz zusammengezogen hat — lieber zwei kurze Aussagen als eine lange. Nenne "
        "ALLE, die der Text hergibt, nicht nur die erste.\n"
        "Zu jeder Aussage gehört ein `beleg`: ein WÖRTLICHES Zitat aus der Nachricht, Zeichen für "
        "Zeichen. Erfinde und paraphrasiere es nicht.\n"
        "NICHT aufnehmen: (1) Fragen des Nutzers — eine Frage ist keine Tatsache. (2) Hypothesen "
        "mit 'wenn/angenommen/wäre'. (3) Abwesenheiten, die er nicht ausdrücklich nennt: aus 'ich "
        "bin Arbeitnehmer' folgt NICHT, dass er keine Kapitalerträge hat.\n"
        "Du ordnest hier nichts zu und rechnest nichts aus — keine Feldnamen, keine Steuerbegriffe, "
        "die im Text nicht stehen. Nur zerlegen.\n"
        "Antworte AUSSCHLIESSLICH mit einem JSON-OBJEKT der Form "
        "{\"aussagen\": [{\"text\":\"…\",\"beleg\":\"Zitat\"}]}. Nichts erkennbar → "
        "\"aussagen\": []. Kein Fließtext außerhalb des JSON.")
    return [{"role": "system", "content": system}, {"role": "user", "content": freitext}]


def _aussagen_parse(text: str, freitext: str) -> list[dict]:
    """Roher LLM-Text → [{text, beleg, status, regeln}]. Kaputtes JSON → [].

    Ein Beleg, der nicht wörtlich im Freitext steht, wird GELEERT, die Aussage aber BEHALTEN. Das ist
    die andere Entscheidung als in Stufe 3, und mit Absicht: dort entsteht aus dem Beleg ein
    gespeicherter Wert, hier nur eine Zwischenüberschrift. Die Aussage wegzuwerfen hiesse, den
    Sachverhalt still zu verlieren — genau das, was diese Stufen abstellen sollen. Ohne Beleg wird sie
    eben ohne Zitat angezeigt.

    DER `text` GEHT NOCH EINMAL DURCH DEN FILTER, und das ist KEIN zweiter Durchgang über die
    Nutzereingabe (die läuft genau einmal, in `_llm_dialog`). Es ist der erste über eine MODELLAUSGABE:
    der Satz ist frei formuliert, und er verlässt in den Stufen 2 und 3 wieder das Haus. Das Argument
    „das Modell kann nichts zurückschreiben, was es nie gesehen hat" trägt — es ist nur ein Argument
    über ein fremdes System, und der Preis, es nicht zu brauchen, ist ein Funktionsaufruf. Der Beleg
    braucht ihn nicht: er muss ohnehin ein Ausschnitt des bereits gefilterten Textes sein."""
    try:
        j = json.loads(text)
    except Exception:                                        # noqa: BLE001
        return []
    if not isinstance(j, dict) or not isinstance(j.get("aussagen"), list):
        return []
    heuhaufen = _normalisiert(freitext)
    out = []
    for a in j["aussagen"]:
        if not isinstance(a, dict):
            continue
        satz = filtere(str(a.get("text", "")).strip()[:300])[0]
        if not satz:
            continue
        beleg = str(a.get("beleg", ""))[:300]
        n = _normalisiert(beleg)
        out.append({"text": satz, "beleg": beleg if len(n) >= 3 and n in heuhaufen else "",
                    "status": "offen", "regeln": []})
    return out


# ============================================================== STUFE 2: welches Thema ist das
#
# Die Zwischenstufe, die den Katalog erst klein macht: 62 Regeln statt 321 Felder, und je Regel eine
# Zeile statt eines Absatzes. Sie nennt KEINE Werte und KEINE Feldnamen — wer hier schon Felder
# vergäbe, hätte die grosse Liste bloss an eine andere Stelle verschoben.
ZUORDNUNG_SCHEMA = {
    "name": "zuordnung",
    "strict": True,
    "schema": {
        "type": "object",
        "properties": {
            "zuordnungen": {
                "type": "array",
                "description": "Ein Eintrag je Aussage, die zu mindestens einer Regel passt.",
                "items": {
                    "type": "object",
                    "properties": {
                        "aussage": {"type": "integer",
                                    "description": "Nummer der Aussage aus der Liste. Für die FRAGE "
                                                   "des Nutzers (keine Aussage): -1."},
                        "regeln": {"type": "array", "items": {"type": "string"},
                                   "description": "Eine oder mehrere Regel-Kennungen, exakt wie "
                                                  "unten geschrieben."},
                    },
                    "required": ["aussage", "regeln"],
                    "additionalProperties": False,
                },
            },
        },
        "required": ["zuordnungen"],
        "additionalProperties": False,
    },
}

_REGEL_FRAGEN = 3            # so viele Feldfragen umreissen eine Regel
_REGEL_FRAGE_ZEICHEN = 44


def _felder_je_regel(katalog: list[dict]) -> dict[str, list[dict]]:
    """Katalog → {regel_id: [Feld, …]}. Felder ohne `regel_id` sammeln sich unter "".

    ABGELEITET, nicht danebengelegt: die Regel-Liste für Stufe 2 und die Feld-Liste für Stufe 3
    stammen aus DEMSELBEN Katalog, den der Aufrufer übergibt. Zwei Listen mit zwei Regeln für
    dieselbe Frage sind in diesem Haus schon mehrfach auseinandergelaufen (api.py:995 hält einen
    solchen Fall fest) — hier gibt es deshalb nur eine Quelle.

    Der ""-Topf ist die ehrliche Behandlung für ein Feld, das sich nicht einordnen lässt: es geht in
    Stufe 3 IMMER mit. Es stillschweigend wegzufiltern hiesse, es unerreichbar zu machen, ohne dass
    es jemand merkt."""
    je: dict[str, list[dict]] = {}
    for f in katalog:
        je.setdefault(str(f.get("regel_id") or ""), []).append(f)
    return je


def _regel_zeilen(je_regel: dict[str, list[dict]], regeln: list[str]) -> str:
    """Eine Zeile je Regel: Kennung + die ersten Feldfragen, gekürzt.

    Die Kennung allein (`p2_festzusetzung_einzel`, `p33_1_2_agb_abzug`) wäre 1.664 Zeichen für alle
    62 und damit fast umsonst — aber sie verlangt vom Modell, unsere Abkürzungen zu erraten. Mit drei
    Feldfragen daneben sind es rund 7.000 Zeichen, immer noch ein Dreizehntel der Feldliste, und die
    Zeile sagt in Nutzersprache, worum es geht. Gekürzt wird an der Wortgrenze: ein mitten im Wort
    abgeschnittener Fragetext liest sich wie ein anderer."""
    zeilen = []
    for r in regeln:
        fragen = []
        for f in je_regel[r][:_REGEL_FRAGEN]:
            t = str(f.get("fragetext_laie") or f.get("feld_id") or "").rstrip("? ").strip()
            if len(t) > _REGEL_FRAGE_ZEICHEN:
                t = t[:_REGEL_FRAGE_ZEICHEN].rsplit(" ", 1)[0] + "…"
            if t:
                fragen.append(t)
        zeilen.append(f"- {r}: {'; '.join(fragen)}" if fragen else f"- {r}")
    return "\n".join(zeilen)


def _mit_zaehlfeldern(kat3: list[dict], katalog: list[dict]) -> list[dict]:
    """Legt das Zählfeld einer Instanz-Gruppe dazu, wenn Felder dieser Gruppe im Katalog stehen.

    Stufe 2 wählt THEMEN, Stufe 3 sieht nur die Felder der gewählten Themen. Das Zählfeld einer
    Instanz-Gruppe liegt aber ausdrücklich in einer ANDEREN Regel — bei `kind` gehört
    `fam_anzahl_kinder` zu `p24b_entlastungsbetrag` (Entlastungsbetrag für Alleinerziehende),
    die Kind-Felder zu `p32_6_kinderfreibetraege`. Das steht so in bindung_regel_bedingungen.yaml
    und war beim Bau der Instanz-Achse als blosse Notiz vermerkt.

    Julius, 2026-08-25, mit „verheiratet, 2 kinder" im Chat: das Modell nahm das Thema
    Kinderfreibeträge (dessen erste Frage nach dem Vornamen fragt — genau die kam) und NICHT das
    Thema Alleinerziehende, denn er ist verheiratet. Die Zahl 2 hatte damit kein Feld, in das sie
    gehen konnte. Folge: ein einziges Vornamensfeld statt zwei, und die Existenzfrage nach Kindern
    stand danach noch in der Ankreuzliste.

    Die Zuordnung wird NICHT geraten: `instanz_gruppen` sagt je Gruppe, welches Feld die Zahl
    trägt. Fehlt die Gruppe dort (sieben der acht), passiert hier nichts.
    """
    gruppen = TR.lade_instanz_gruppen()
    if not gruppen:
        return kat3
    drin = {f.get("feld_id") for f in kat3}
    gebraucht = {gruppen[g]["anzahl_feld"] for f in kat3
                 for g in [f.get("instanz_gruppe")] if g and g in gruppen}
    fehlend = gebraucht - drin
    if not fehlend:
        return kat3
    return kat3 + [f for f in katalog if f.get("feld_id") in fehlend]


def _themen_prompt(freitext: str, aussagen: list[dict], regel_zeilen: str) -> list[dict]:
    """Stufe 2: Aussagen → Regel-Kennungen. Der Nutzertext geht MIT hinaus, und zwar für den Fall,
    dass Stufe 1 nichts fand: eine reine Frage ist keine Tatsachenaussage, hat aber sehr wohl ein
    Thema — und die Felder dieses Themas machen den Unterschied zwischen einer Antwort aus
    Allgemeinwissen und einer aus unserem Fragebogen."""
    nummeriert = "\n".join(f"[{i}] {a['text']}" for i, a in enumerate(aussagen)) or "(keine)"
    system = (
        "Du ordnest Aussagen über einen Steuerpflichtigen den THEMEN unserer Steuererklärung zu. Ein "
        "Thema ist eine der Regeln unten; jede Zeile nennt die Regel-Kennung und, wonach sie fragt.\n"
        "Du nennst KEINE Werte, KEINE Beträge und KEINE Feldnamen — ausschliesslich Regel-Kennungen, "
        "exakt so geschrieben wie unten.\n"
        "Ordne JEDE Aussage zu, und ruhig MEHREREN Regeln: 'seit Juli arbeitslos' betrifft sowohl "
        "den Arbeitslohn bis Juli als auch die Lohnersatzleistung danach. Lieber eine Regel zu viel "
        "als eine zu wenig — was hier fehlt, wird später nicht mehr gefragt. Passt beim besten "
        "Willen keine, lass die Aussage weg, statt zu raten.\n"
        "Hat der Nutzer eine FRAGE gestellt, ordne auch ihr Thema zu und schreib dafür aussage: -1.\n\n"
        "AUSSAGEN:\n" + nummeriert + "\n\nREGELN:\n" + regel_zeilen + "\n\n"
        "Antworte AUSSCHLIESSLICH mit einem JSON-OBJEKT der Form "
        "{\"zuordnungen\": [{\"aussage\":0,\"regeln\":[\"…\"]}]}. Kein Fließtext außerhalb des JSON.")
    return [{"role": "system", "content": system}, {"role": "user", "content": freitext}]


def _zuordnung_parse(text: str, erlaubt: set[str], anzahl: int) -> tuple[dict[int, list[str]], list[str]]:
    """Roher LLM-Text → ({aussage_nr: [regel_id]}, alle getroffenen regel_ids).

    Erfundene Kennungen fallen weg — eine Regel, die es nicht gibt, kann in Stufe 3 keine Felder
    beitragen. Die getroffenen Regeln werden dagegen UNABHÄNGIG von der Aussage-Nummer gesammelt:
    eine unbrauchbare Nummer (etwa die -1 für die Frage des Nutzers) darf die Verengung des Katalogs
    nicht verhindern. Die Nummer entscheidet nur, welcher Aussage der Treffer zugerechnet wird."""
    try:
        j = json.loads(text)
    except Exception:                                        # noqa: BLE001
        return {}, []
    if not isinstance(j, dict) or not isinstance(j.get("zuordnungen"), list):
        return {}, []
    je_aussage: dict[int, list[str]] = {}
    getroffen: list[str] = []
    for z in j["zuordnungen"]:
        if not isinstance(z, dict) or not isinstance(z.get("regeln"), list):
            continue
        regeln = [r for r in (str(x) for x in z["regeln"]) if r in erlaubt]
        if not regeln:
            continue
        for r in regeln:
            if r not in getroffen:
                getroffen.append(r)
        i = _index(z.get("aussage"))
        if i is not None and i < anzahl:
            je_aussage.setdefault(i, [])
            je_aussage[i] += [r for r in regeln if r not in je_aussage[i]]
    return je_aussage, getroffen


# ============================================================== Was aus jeder Aussage geworden ist

def _zugerechnet(eintraege: list[dict], aussagen: list[dict]) -> set[int]:
    """Nummern der Aussagen, auf die mindestens ein Eintrag (Vorschlag/Rückfrage) zeigt.

    Zwei Wege, und der zweite ist der wichtige: die Nummer aus der Modellantwort, und — wenn sie
    fehlt — der Beleg. Ein Modell, das `aussage: -1` schreibt, obwohl sein Zitat wörtlich aus einer
    Aussage stammt, liesse sonst eine verwertete Aussage als „nirgends angekommen" erscheinen. Ein
    Fehlalarm ist hier nicht harmlos: er würde dem Nutzer sagen, etwas sei verlorengegangen, das
    längst in seinem Fall steht."""
    treffer = set()
    belege = [_normalisiert(a["beleg"]) for a in aussagen]
    for e in eintraege:
        i = e.get("aussage")
        if i is not None and i < len(aussagen):
            treffer.add(i)
            continue
        eigen = _normalisiert(e.get("beleg", ""))
        if len(eigen) < 3:
            continue
        for k, b in enumerate(belege):
            if len(b) >= 3 and (b in eigen or eigen in b):
                treffer.add(k)
                break
    return treffer


def _status_setzen(aussagen, zuordnungen, behalten, verworfen, rueckfragen) -> None:
    """Jede Aussage bekommt ihr Ergebnis — auch das Ergebnis „nichts".

    DER PUNKT DER GANZEN ÜBUNG. Bisher fiel eine Angabe, für die das Modell kein Feld fand, ersatzlos
    weg; der Nutzer sah nur, dass etwas fehlt, und wir sahen es gar nicht. Ab hier ist eine Aussage
    ohne Ziel ein benanntes Ergebnis: `kein_thema` (Stufe 2 fand keine Regel), `kein_feld` (Regel ja,
    Feld nein) oder `ohne_beleg` (das Beleg-Gate hat den Vorschlag verworfen). Drei Gründe, und sie
    verlangen Verschiedenes — der erste ist unsere Lücke, der dritte ein Modellfehler."""
    mit_vorschlag = _zugerechnet(behalten, aussagen)
    mit_rueckfrage = _zugerechnet(rueckfragen, aussagen)
    ohne_beleg = _zugerechnet(verworfen, aussagen)
    for i, a in enumerate(aussagen):
        a["regeln"] = zuordnungen.get(i, [])
        if i in mit_vorschlag:
            a["status"] = "vorschlag"
        elif i in mit_rueckfrage:
            a["status"] = "rueckfrage"
        elif i in ohne_beleg:
            a["status"] = "ohne_beleg"
        else:
            a["status"] = "kein_thema" if not a["regeln"] else "kein_feld"


def _teilergebnis(aussagen: list[dict], zuordnungen: dict, status: str) -> dict:
    """Was der Nutzer bekommt, wenn Stufe 2 oder 3 ausfällt: die Aussagen, die schon dastehen.

    Die Auflage dahinter: ein Ausfall NACH Stufe 1 darf nicht dazu führen, dass der Nutzer gar nichts
    sieht. Er hat fünf Dinge geschrieben, wir haben fünf verstanden — das ist mitteilbar, auch ohne
    einen einzigen Vorschlag. `antwort` bleibt LEER: eine erfundene Ersatzantwort wäre an dieser
    Stelle das Gegenteil von ehrlich, und `unsicher=false` daneben eine Behauptung über eine Antwort,
    die es nicht gibt."""
    for i, a in enumerate(aussagen):
        a["regeln"] = zuordnungen.get(i, [])
        a["status"] = status
    return {"vorschlaege": [], "antwort": "", "unsicher": False,
            "aussagen": aussagen, "rueckfragen": [], "rueckfragen_zurueckgestellt": 0}


def _llm_dialog(freitext: str, katalog: list[dict], kontext: str = "",
                user_id: str | None = None) -> dict:
    """{vorschlaege, antwort, unsicher, aussagen, rueckfragen} — DREI Aufrufe, drei Fragen.

    1. WAS HAT ER GESAGT   (kein Katalog)          → Aussagen mit wörtlichem Beleg
    2. WELCHES THEMA IST DAS (62 Regeln, 1 Zeile)  → Regel-Kennungen je Aussage
    3. WELCHER WERT IST DAS  (nur die Felder 2.)   → Vorschläge, Rückfragen, Antwort

    Vorher war es EIN Aufruf mit allen 321 Feldern. Gemessen an Julius' Nachricht (231 Zeichen,
    fünf Tatsachen): 96.679 Zeichen System-Prompt, davon 96 % Feldliste, und über vier Läufe stabil
    3 von 5 Tatsachen erkannt — „seit Juli arbeitslos" und „Ausgaben für die Gesundheit" nie.

    Die ersten drei Rückgabeschlüssel sind UNVERÄNDERT (api.py:chat und die Oberfläche hängen
    daran); `aussagen` und `rueckfragen` kommen additiv dazu.

    Dialog-Task-Wrapper (Handler-Schicht) ÜBER llm_client.complete (der einen niedrig-level
    Wahrheit). Cap-gated: kein Key/Base/Modell → LlmNichtVerfuegbar propagiert (der /chat-Handler
    fängt sie → 501). Der Aufrufer schreibt jeden Vorschlag als VORLÄUFIGES Event (Store-Auflage A
    + Katalog-Check erzwingen die Sicherheit); der Mensch bestätigt einzeln.

    AUSFALL JE STUFE — drei Aufrufe sind drei Ausfallpunkte, und sie verlangen Verschiedenes:
      Stufe 1 → die Ausnahme propagiert (501, wie bisher). Es gibt nichts zu retten, und der alte
        Ein-Aufruf-Weg als Rückfall wäre ausgerechnet die 96.679 Zeichen, die hier weg sollten.
      Stufe 2 → die Aussagen gehen zurück, ohne Vorschläge. KEIN Versuch mit dem vollen Katalog:
        Stufe 2 ist der mit Abstand billigste Aufruf: fällt ausgerechnet er aus, ist der grösste
        das Letzte, womit man einem angeschlagenen Anbieter kommen sollte. Ein Neuversuch an Ort
        und Stelle wäre ohnehin sinnlos — `leere_antwort` hat der Client schon dreimal probiert,
        und `abgeschnitten` läuft bei temperature=0 in dieselbe Grenze.
      Stufe 3 → die Aussagen samt ihrer Regel-Zuordnung gehen zurück, ohne Vorschläge.
    In allen drei Fällen steht der Grund im Protokoll, mit der Stufe daneben.

    KATALOG-RÜCKFALL: findet Stufe 2 keine einzige Regel, sieht Stufe 3 den VOLLEN Katalog. Die
    Verengung ist eine Sparmassnahme, keine Befugnis — was geschrieben werden darf, entscheidet
    unverändert der Katalog-Check im Store (fail-closed, global). Eine misslungene Verengung darf
    den Nutzer deshalb nicht seine Vorschläge kosten; sie kostet dann eben wieder Tokens, und im
    Protokoll steht `katalog=voll`, damit sichtbar ist, wie oft das passiert.

    Die `antwort` durchläuft KEIN Beleg-Gate — sie behauptet ja nichts über den Nutzer, sondern
    erklärt ihm etwas. Der Filter gilt genau dort, wo aus Text ein gespeicherter Wert würde.

    PII-Filter: Vor dem ausgehenden LLM-Call werden personenbezogene Daten (IdNr, IBAN, Datum,
    PLZ/Ort, Straße, Anrede+Name) maskiert — im Freitext UND im Kontext, der die schon bestätigten
    Angaben trägt. Geldbeträge und Paragraphen bleiben unangetastet.
    Audit: pro Call ein Eintrag mit Kategorien + Längen (NIEMALS der Freitext selbst). Dazu der
    ANBIETER, der geantwortet hat: bei einem Vermittler wie OpenRouter bedient dieselbe
    Modell-Kennung viele Endpunkte, und ob unser JSON-Schema erzwungen oder stillschweigend
    ignoriert wird, hängt am Endpunkt. Ohne den Namen ist die Absicherung im Client
    (`provider.require_parameters`) nicht nachprüfbar. Ein Anbietername ist ein Metadatum — der
    Antworttext bleibt draussen, wie jeder Text."""
    leer = {"vorschlaege": [], "antwort": "", "unsicher": False, "aussagen": [],
            "rueckfragen": [], "rueckfragen_zurueckgestellt": 0}
    if not (freitext or "").strip():
        return leer
    # GENAU EINMAL gefiltert, und zwar hier. Die Stufen 2 und 3 arbeiten auf dem Ergebnis von
    # Stufe 1 bzw. auf `gefiltert` — der Rohtext verlässt dieses Haus an keiner der drei Stellen.
    # Die Aussagen aus Stufe 1 sind aus dem gefilterten Text gebildet; was der Filter entfernt hat,
    # hat das Modell nie gesehen und kann es folglich nicht zurückschreiben.
    gefiltert, kategorien = filtere(freitext)
    kontext_gefiltert, kategorien_k = filtere(kontext) if kontext else ("", [])
    import llm_client
    kopf = (f"pii_kategorien={kategorien}, kontext_kategorien={kategorien_k}, "
            f"textlaenge_vor={len(freitext)}, textlaenge_nach={len(gefiltert)}")

    def melde(teil: str) -> None:
        """Ein Protokolleintrag je Stufe. NUR Metadaten, nie ein Zeichen Inhalt — `kopf` steht in
        jedem, damit ein Eintrag für sich lesbar bleibt (produkt/store/audit.py, test_pii_filter)."""
        audit.append(user_id or "unbekannt", "llm_call", None, f"{kopf}, {teil}")

    def mitschnitt(stufe: int, was: str, inhalt) -> None:
        """Wortlaut der Modellantwort — NUR wenn `TAXGRAPH_KI_DEBUG=1` gesetzt ist.

        WARUM DAS NICHT INS AUDIT GEHÖRT und nicht standardmässig läuft: hier steht der INHALT,
        also alles, was der PII-Filter nicht erwischt hat — Personennamen zuallererst (die erkennt
        er bewusst nicht, s. pii_filter.py). Das Audit-Protokoll führt seit jeher ausschliesslich
        Metadaten, und tests/test_pii_filter.py erzwingt das; ein Mitschnitt dort wäre kein
        Debug-Werkzeug, sondern ein zweiter Datenspeicher ohne Zweckbindung.

        ANLASS (Julius, 2026-08-23): "bitte die ki antworten fuer das debugging anstaendig loggen".
        Der Anlass ist konkret — im Verlauf jenes Tages war weder nachvollziehbar, WELCHE
        Rückfragen gestellt wurden noch WAS die KI geantwortet hat. Das Audit führt nur ihre
        Anzahl (`rueckfragen=5`, `antwortlaenge=246`); der Wortlaut existierte ausschliesslich im
        Browserfenster und war nach einem Neuladen weg. Eine Diagnose ohne den Wortlaut ist
        Ratearbeit — genau deshalb gibt es das hier, und genau deshalb ist es abschaltbar.

        Ablage neben dem Audit (über `audit.AUDIT_DIR`, zur AUFRUFZEIT gelesen): dieselbe
        Wegbeschreibung, damit Tests, die die Ablage umlenken, auch diese Datei mitnehmen. Ein
        `from audit import AUDIT_DIR` bände den Wert statt des Namens und liefe an jeder Umlenkung
        vorbei. Rechte 0600 wie beim Audit — die Datei führt den Klartext einer Steuererklärung.
        """
        # Seit 2026-08-27 in DENSELBEN Strang wie Fragen und Antworten (produkt/haut/flow.py).
        # Vorher lag der Wortlaut der Modellstufen in einer eigenen Datei, und der Fragebogen —
        # wo der Nutzer die meiste Zeit verbringt — kam darin gar nicht vor. Julius: „ich will so
        # ein log wo der ganze flow nachvollziehbar ist." Zwei Dateien nebeneinander sind kein
        # Fluss; die Reihenfolge zwischen ihnen musste man sich aus Zeitstempeln zusammenlegen.
        flow.schreibe(None, "ki", {"stufe": stufe, "was": was, "inhalt": inhalt})

    def gescheitert(stufe: int, e: Exception) -> None:
        # Ohne diesen Eintrag bliebe im Protokoll GAR NICHTS stehen — ein Aufruf, der leer zurückkam
        # und nach drei Versuchen aufgab, wäre von einem, der nie stattfand, nicht zu unterscheiden.
        # `grund` ist ein Wort aus dem kontrollierten Vokabular des Clients, NICHT die Meldung: die
        # trägt gekürzten Anbietertext und gehört nicht in ein Metadaten-Protokoll. Die STUFE steht
        # daneben, weil sonst wieder nur „der Aufruf ging schief" im Protokoll stünde — und genau
        # das war der Zustand, in dem niemand sagen konnte, wo Julius' drei Fakten blieben.
        melde(f"stufe={stufe}, ergebnis=kein_ergebnis, "
              f"grund={getattr(e, 'grund', '') or 'sonstiger_fehler'}, "
              f"provider={llm_client.letzte_meta().get('provider', '')!r}")

    # --------------------------------------------------------- Stufe 1: was hat er gesagt
    try:
        c1 = llm_client.complete("chat", _aussagen_prompt(gefiltert), schema=AUSSAGEN_SCHEMA)
    except llm_client.LlmNichtVerfuegbar as e:
        gescheitert(1, e)
        raise                                   # der Aufrufer sieht wie bisher nur die Ausnahme
    aussagen = _aussagen_parse(c1.text, gefiltert)
    mitschnitt(1, "aussagen", aussagen)
    melde(f"stufe=1, aussagen={len(aussagen)}, "
          f"aussagen_ohne_beleg={sum(1 for a in aussagen if not a['beleg'])}, "
          f"inhalt_laenge={len(c1.text or '')}, provider={c1.provider!r}, finish={c1.finish!r}")

    # --------------------------------------------------------- Stufe 2: welches Thema ist das
    je_regel = _felder_je_regel(katalog)
    verfuegbar = sorted(r for r in je_regel if r)
    zuordnungen: dict[int, list[str]] = {}
    getroffen: list[str] = []
    if verfuegbar:
        try:
            c2 = llm_client.complete(
                "chat", _themen_prompt(gefiltert, aussagen, _regel_zeilen(je_regel, verfuegbar)),
                schema=ZUORDNUNG_SCHEMA)
        except llm_client.LlmNichtVerfuegbar as e:
            gescheitert(2, e)
            return _teilergebnis(aussagen, {}, "themen_ausgefallen")
        zuordnungen, getroffen = _zuordnung_parse(c2.text, set(verfuegbar), len(aussagen))
        mitschnitt(2, "zuordnungen", {"zuordnungen": zuordnungen, "regeln": sorted(getroffen)})
        melde(f"stufe=2, aussagen={len(aussagen)}, zugeordnet={len(zuordnungen)}, "
              f"regeln={len(getroffen)}/{len(verfuegbar)}, inhalt_laenge={len(c2.text or '')}, "
              f"provider={c2.provider!r}, finish={c2.finish!r}")

    # --------------------------------------------------------- Stufe 3: welcher Wert ist das
    eng = bool(getroffen)
    kat3 = ([f for r in getroffen for f in je_regel[r]] + je_regel.get("", [])) if eng else katalog
    kat3 = _mit_zaehlfeldern(kat3, katalog)
    try:
        c3 = llm_client.complete("chat",
                                 _dialog_prompt(gefiltert, kat3, kontext_gefiltert, aussagen),
                                 schema=DIALOG_SCHEMA)
    except llm_client.LlmNichtVerfuegbar as e:
        gescheitert(3, e)
        return _teilergebnis(aussagen, zuordnungen, "werte_ausgefallen")
    # Beleg-Gate: nur Vorschläge mit wörtlichem Zitat aus DEM Text, den das Modell gesehen hat —
    # dem gefilterten NUTZERTEXT, nicht den Aussagen aus Stufe 1. Das ist der Unterschied, an dem
    # das Gate hängt: eine Aussage ist selbst Modellausgabe, und ein Beleg, der gegen sie geprüft
    # würde, belegte eine Modellausgabe mit einer anderen. Das Modell kann eine Begründung
    # erfinden, aber kein Zitat, das in der Nachricht des Nutzers nicht vorkommt.
    # NUR das Beleg-Gate. Ein `rechenweg` wird NICHT nachgerechnet und der Vorschlag deshalb auch
    # nicht verworfen — Julius' Entscheidung vom 2026-08-23: "sollten wir nicht dem modell zutrauen
    # diese rechnung zu können und der user bestätigt. wenn das problematisch werden sollte können
    # wir nochmal nachbessern."
    #
    # Der erste Entwurf rechnete nach und warf bei Abweichung weg. Das war aus zwei Gründen falsch:
    # ein verworfener Vorschlag ist für den Nutzer UNSICHTBAR — genau der stille Verlust, gegen den
    # der Aussagen-Status am selben Tag gebaut wurde —, und es widerspricht der Grundregel dieses
    # Hauses (Julius-Entscheid 2026-08-14): die KI schlägt vor, der Mensch bestätigt jedes Feld.
    # Eine Multiplikation, die der Nutzer neben dem Rechenweg stehen sieht, kann er selbst prüfen;
    # ihm den Vorschlag vorher wegzunehmen, nimmt ihm die Entscheidung.
    #
    # `rechenweg` bleibt im Schema und wandert MIT dem Vorschlag nach oben — die Oberfläche zeigt
    # ihn unter dem Wert ("50.000 € pro Jahr ÷ 12 × 6 Monate"). Er ist damit Anzeige, nicht Gate.
    behalten, verworfen = _beleg_geprueft(_chat_parse(c3.text), gefiltert)
    rueckfragen = _rueckfragen_parse(c3.text, len(kat3))
    # ERST bündeln, DANN verdrängen. Andersherum nähme eine Rückfrage, die gleich wegfällt, den
    # Vorschlag zu ihrem Feld mit — der Nutzer verlöre den Wert UND die Frage danach, und zwar
    # lautlos. Genau die Bauart von stillem Verlust, gegen die die Aussagen-Liste gebaut wurde.
    rueckfragen, zurueckgestellt = _rueckfragen_gebuendelt(rueckfragen)
    behalten = _rueckfrage_verdraengt(behalten, rueckfragen)
    antwort, unsicher = _antwort_parse(c3.text)
    _status_setzen(aussagen, zuordnungen, behalten, verworfen, rueckfragen)
    # Nur Metadaten, nie den Freitext (roh oder gefiltert). `ohne_beleg_verworfen` misst, wie oft
    # das Modell etwas behauptet, das im Text nicht steht; `offen` misst das Gegenstück dazu — wie
    # oft WIR für etwas, das der Nutzer gesagt hat, kein Feld hatten. `inhalt_laenge` unterscheidet
    # „das Modell hatte nichts zu sagen" von „es kam etwas, das wir nicht verwerten konnten", ohne
    # dass ein Zeichen des Inhalts ins Protokoll wandert.
    mitschnitt(3, "ergebnis", {"vorschlaege": behalten, "ohne_beleg_verworfen": verworfen,
                               "rueckfragen": rueckfragen, "antwort": antwort,
                               "unsicher": unsicher, "aussagen": aussagen})
    melde(f"stufe=3, katalog={'eng' if eng else 'voll'}, katalog_felder={len(kat3)}, "
          f"vorschlaege={len(behalten)}, ohne_beleg_verworfen={len(verworfen)}, "
          f"rueckfragen={len(rueckfragen)}, rueckfragen_zurueckgestellt={zurueckgestellt}, "
          f"offen={sum(1 for a in aussagen if a['status'] not in ('vorschlag', 'rueckfrage'))}, "
          f"antwortlaenge={len(antwort)}, unsicher={unsicher}, "
          f"inhalt_laenge={len(c3.text or '')}, provider={c3.provider!r}, finish={c3.finish!r}")
    return {"vorschlaege": behalten, "antwort": antwort, "unsicher": unsicher,
            "aussagen": aussagen, "rueckfragen": rueckfragen,
            "rueckfragen_zurueckgestellt": zurueckgestellt}


def _kontoauszug_llm_klassifikator():
    """Baut den Kontoauszug-LLM-Fallback-Klassifikator (dev-2s kontoauszug_writer.llm_klassifikator_factory,
    llm_client-MODUL als `client` — hat `.complete`, kein Klassen-Bau nötig). Cap-gated wie /chat: JEDER Aufruf
    fängt NUR LlmNichtVerfuegbar (Cap-Gate/Netzfehler — die Factory selbst fängt nichts) und liefert None (=
    unklassifiziert, wie bisher llm_klassifikator=None) statt den GESAMTEN Upload bei der ERSTEN mehrdeutigen
    Transaktion abstürzen zu lassen (Regression ggü. det-only). Ein Logik-/Parse-Bug ist KEIN erwarteter
    Cap-Gate-Fall — der propagiert bewusst (K2: silent-swallow eines echten Bugs ist selbst ein Risiko)."""
    import llm_client
    roh = KW.llm_klassifikator_factory(llm_client, "kontoauszug_klassifikation")

    def klassifikator(zweck, betrag):
        try:
            return roh(zweck, betrag)
        except llm_client.LlmNichtVerfuegbar:
            return None
    return klassifikator
