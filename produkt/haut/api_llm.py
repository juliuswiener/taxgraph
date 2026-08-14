"""LLM-Integration für Chat-Berater + Kontoauszug-Klassifikation (Bright-Line Isolation).

Dieses Modul ISOLIERT den llm_client-Import auf eine Datei (hier). api.py + Server
importieren diese Funktionen, ohne selbst llm_client zu kennen. Lazy imports innerhalb
der Funktionen halten die Abhängigkeit locker. Exportiert LlmNichtVerfuegbar damit
api.py sie in except-Clauses nutzen kann."""

import json
import audit  # noqa: E402 — P1.6 Audit-Log (sys.path via api.py)
import kontoauszug_writer as KW
from pii_filter import filtere  # noqa: E402 — PII-Filter vor ausgehendem LLM-Call

# Exception für Exception-Handling in api.py (ohne dass api.py selbst llm_client importiert)
try:
    import llm_client
    LlmNichtVerfuegbar = llm_client.LlmNichtVerfuegbar
except ImportError:
    # Fallback wenn llm_client nicht da (dev-Umgebung)
    class LlmNichtVerfuegbar(Exception):
        pass


def _dialog_prompt(freitext: str, katalog: list[dict], kontext: str = "") -> list[dict]:
    """Baut die OpenAI-kompatible messages-Liste für den EINEN Dialog-Task: aus einem Nutzersatz
    Feld-Werte VORSCHLAGEN **und** eine Frage BEANTWORTEN, in einem Aufruf.

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
        "TATSACHEN ÜBERSETZT DU IMMER: was der Nutzer als Tatsache über sich sagt, wird zum "
        "Vorschlag — auch dann, wenn das Feld eine WAHL abbildet und der Text nur die übliche Wahl "
        "nahelegt. Vorgeschlagen ist nicht gesetzt: der Mensch sieht den Vorschlag neben deinem "
        "Zitat und bestätigt oder ändert ihn. Zurückhaltung gilt NUR für die beiden Fälle oben "
        "(kein_-Felder ohne ausgesprochene Abwesenheit, und Hypothesen).\n\n"
        + (kontext + "\n\n" if kontext else "")
        + "Für die ANTWORT: schreib auf Deutsch, in der Du-Form, höchstens fünf Sätze, ohne "
        "Fachjargon (ein unvermeidbares Fachwort erklärst du im selben Satz). Stütze dich auf den "
        "oben zitierten Gesetzestext, falls einer angegeben ist. Nenne NIE einen konkreten Betrag "
        "als den Wert des Nutzers — er trägt jeden Wert selbst ein. Bist du dir nicht sicher, sag "
        "das ausdrücklich und setze \"unsicher\": true, statt zu raten. Hat der Nutzer gar nichts "
        "gefragt, ist `antwort` ein LEERER String — dann erfinde keine Belehrung.\n"
        "Antworte AUSSCHLIESSLICH mit einem JSON-OBJEKT der Form "
        "{\"vorschlaege\": [{\"feld_id\":\"…\",\"wert\":…,\"beleg\":\"Zitat\",\"begruendung\":\"kurz\"}], "
        "\"antwort\":\"…\", \"unsicher\":false}. "
        "Die Liste enthält EINEN Eintrag JE FELD, für das die Beschreibung einen konkreten Wert "
        "hergibt — nenne alle, die du erkennst, nicht nur den ersten. Kein Treffer → "
        "\"vorschlaege\": []. Kein Fließtext außerhalb des JSON.")
    return [{"role": "system", "content": system}, {"role": "user", "content": freitext}]


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
                    },
                    "required": ["feld_id", "wert", "beleg", "begruendung"],
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
        "required": ["vorschlaege", "antwort", "unsicher"],
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

    Kurze Belege (< 3 Zeichen) zählen nicht: "5" steht in fast jedem Text und belegt nichts."""
    behalten, verworfen = [], []
    heuhaufen = _normalisiert(freitext)
    for v in vorschlaege:
        beleg = _normalisiert(v.get("beleg", ""))
        if len(beleg) >= 3 and beleg in heuhaufen:
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
                        "begruendung": str(v.get("begruendung", ""))[:200]})
    return out


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


def _llm_dialog(freitext: str, katalog: list[dict], kontext: str = "",
                user_id: str | None = None) -> dict:
    """{vorschlaege, antwort, unsicher} — EIN Aufruf, beide Aufgaben.

    Dialog-Task-Wrapper (Handler-Schicht) ÜBER llm_client.complete (der einen niedrig-level
    Wahrheit). Cap-gated: kein Key/Base/Modell → LlmNichtVerfuegbar propagiert (der /chat-Handler
    fängt sie → 501). Der Aufrufer schreibt jeden Vorschlag als VORLÄUFIGES Event (Store-Auflage A
    + Katalog-Check erzwingen die Sicherheit); der Mensch bestätigt einzeln.

    Die `antwort` durchläuft KEIN Beleg-Gate — sie behauptet ja nichts über den Nutzer, sondern
    erklärt ihm etwas. Der Filter gilt genau dort, wo aus Text ein gespeicherter Wert würde.

    PII-Filter: Vor dem ausgehenden LLM-Call werden personenbezogene Daten (IdNr, IBAN, Datum,
    PLZ/Ort, Straße, Anrede+Name) maskiert — im Freitext UND im Kontext, der die schon bestätigten
    Angaben trägt. Geldbeträge und Paragraphen bleiben unangetastet.
    Audit: pro Call ein Eintrag mit Kategorien + Längen (NIEMALS der Freitext selbst)."""
    leer = {"vorschlaege": [], "antwort": "", "unsicher": False}
    if not (freitext or "").strip():
        return leer
    gefiltert, kategorien = filtere(freitext)
    kontext_gefiltert, kategorien_k = filtere(kontext) if kontext else ("", [])
    import llm_client
    comp = llm_client.complete("chat", _dialog_prompt(gefiltert, katalog, kontext_gefiltert),
                               schema=DIALOG_SCHEMA)
    roh = _chat_parse(comp.text)
    # Beleg-Gate: nur Vorschläge mit wörtlichem Zitat aus DEM Text, den das Modell gesehen hat.
    # Das Modell kann eine Begründung erfinden, aber kein Zitat, das im Text nicht vorkommt.
    behalten, verworfen = _beleg_geprueft(roh, gefiltert)
    antwort, unsicher = _antwort_parse(comp.text)
    # Audit: nur Metadaten, nie den Freitext (roh oder gefiltert). Die Zahl der belegfrei
    # verworfenen Vorschläge gehört dazu — sie ist das Maß dafür, wie oft das Modell etwas
    # behauptet, das im Text nicht steht.
    audit.append(user_id or "unbekannt", "llm_call", None,
                 f"pii_kategorien={kategorien}, kontext_kategorien={kategorien_k}, "
                 f"textlaenge_vor={len(freitext)}, "
                 f"textlaenge_nach={len(gefiltert)}, vorschlaege={len(behalten)}, "
                 f"ohne_beleg_verworfen={len(verworfen)}, "
                 f"antwortlaenge={len(antwort)}, unsicher={unsicher}")
    return {"vorschlaege": behalten, "antwort": antwort, "unsicher": unsicher}


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
