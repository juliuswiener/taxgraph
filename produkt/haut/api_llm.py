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


def _chat_prompt(freitext: str, katalog: list[dict]) -> list[dict]:
    """Baut die OpenAI-kompatible messages-Liste für den Chat-Vorschlags-Task. System-Regel: die KI darf
    AUSSCHLIESSLICH die Felder aus dem übergebenen Katalog vorschlagen (askable + vorschlagbar; der Store-
    Katalog-Check ist die zweite Verteidigung), NUR als Vorschlag, mit Feld-Metadaten (fragetext/typ/bereich/
    enum). Antwort = striktes JSON. Task-Wrapper (Handler-Schicht) — der Client (llm_client) kennt diesen
    Prompt nicht."""
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
        "Du bist ein Steuer-Assistent, der aus der Freitext-Beschreibung eines Nutzers Feld-Werte VORSCHLÄGT. "
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
        "Antworte AUSSCHLIESSLICH mit einem JSON-OBJEKT der Form "
        "{\"vorschlaege\": [{\"feld_id\":\"…\",\"wert\":…,\"beleg\":\"Zitat\",\"begruendung\":\"kurz\"}]}. "
        "Die Liste enthält EINEN Eintrag JE FELD, für das die Beschreibung einen konkreten Wert "
        "hergibt — nenne alle, die du erkennst, nicht nur den ersten. Kein Treffer → "
        "{\"vorschlaege\": []}. Kein Fließtext.")
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
CHAT_SCHEMA = {
    "name": "feld_vorschlaege",
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
        },
        "required": ["vorschlaege"],
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


def _llm_vorschlaege(freitext: str, katalog: list[dict],
                     user_id: str | None = None) -> list[dict]:
    """Chat-Task-Wrapper (Handler-Schicht) ÜBER llm_client.complete (der einen niedrig-level Wahrheit). Cap-
    gated: kein Key/Base/Modell → LlmNichtVerfuegbar propagiert (der /chat-Handler fängt sie → 501). Der
    Aufrufer schreibt jeden Vorschlag als VORLÄUFIGES Event (Store-Auflage A + Katalog-Check erzwingen die
    Sicherheit); der Mensch bestätigt via Hold-Confirm.

    PII-Filter: Vor dem ausgehenden LLM-Call werden personenbezogene Daten (IdNr, IBAN, Datum,
    PLZ/Ort, Straße, Anrede+Name) maskiert. Geldbeträge und Paragraphen bleiben unangetastet.
    Audit: pro Call ein Eintrag mit Kategorien + Textlänge (NIEMALS der Freitext selbst)."""
    if not (freitext or "").strip():
        return []
    gefiltert, kategorien = filtere(freitext)
    import llm_client
    comp = llm_client.complete("chat", _chat_prompt(gefiltert, katalog), schema=CHAT_SCHEMA)
    roh = _chat_parse(comp.text)
    # Beleg-Gate: nur Vorschläge mit wörtlichem Zitat aus DEM Text, den das Modell gesehen hat.
    # Das Modell kann eine Begründung erfinden, aber kein Zitat, das im Text nicht vorkommt.
    behalten, verworfen = _beleg_geprueft(roh, gefiltert)
    # Audit: nur Metadaten, nie den Freitext (roh oder gefiltert). Die Zahl der belegfrei
    # verworfenen Vorschläge gehört dazu — sie ist das Maß dafür, wie oft das Modell etwas
    # behauptet, das im Text nicht steht.
    audit.append(user_id or "unbekannt", "llm_call", None,
                 f"pii_kategorien={kategorien}, textlaenge_vor={len(freitext)}, "
                 f"textlaenge_nach={len(gefiltert)}, vorschlaege={len(behalten)}, "
                 f"ohne_beleg_verworfen={len(verworfen)}")
    return behalten


# ---------------------------------------------------------------- Erklär-Kanal (Nachfragen)
#
# Zweiter LLM-Task neben den Vorschlägen, mit einer anderen Grenze: hier kommt TEXT zurück und
# NIE ein Feld-Wert. Kein Event, kein Store-Schreibvorgang — der Pfad ruft append_event gar nicht
# erst auf. Der Nutzer fragt nach ("was zählt denn zu den Arbeitstagen?"), das Modell antwortet.
#
# Das Schema hat einen zweiten Zweck neben der Struktur: `unsicher` zwingt das Modell zu einer
# ausdrücklichen Aussage darüber, ob die Antwort aus dem mitgegebenen Gesetzestext folgt. Wer ein
# Feld setzen MUSS, gibt Zweifel eher zu als jemand, der nur höflich darum gebeten wird — und die
# Oberfläche kann den Hinweis dann anzeigen, statt eine Vermutung wie eine Auskunft aussehen zu
# lassen. (Ein response_format geht in _call ohnehin immer raus; ohne Schema wäre es json_object,
# und die Erklärung käme als JSON-Objekt zurück, das niemand liest.)
ERKLAER_SCHEMA = {
    "name": "erklaerung",
    "strict": True,
    "schema": {
        "type": "object",
        "properties": {
            "antwort": {"type": "string",
                        "description": "Die Erklärung: höchstens fünf Sätze, Du-Form, ohne Fachjargon."},
            "unsicher": {"type": "boolean",
                         "description": "true, wenn die Antwort NICHT sicher aus dem angegebenen "
                                        "Gesetzestext oder der Feldbeschreibung folgt."},
        },
        "required": ["antwort", "unsicher"],
        "additionalProperties": False,
    },
}


def _erklaer_prompt(frage: str, kontext: str) -> list[dict]:
    """messages für den Erklär-Task. `kontext` ist der bereits PII-gefilterte Text mit Feld,
    Kurzhilfe, Zitatanker und den schon bestätigten Angaben (baut api.py)."""
    system = (
        "Du erklärst einem Laien eine Frage aus seiner Einkommensteuererklärung. Du ERKLÄRST — du "
        "setzt keinen Wert, triffst keine Entscheidung und rechnest seine Steuer nicht aus.\n\n"
        + kontext + "\n"
        "Antworte auf Deutsch, in der Du-Form, in höchstens fünf Sätzen, ohne Fachjargon (und wenn "
        "ein Fachwort unvermeidlich ist, erkläre es im selben Satz). Stütze dich auf den zitierten "
        "Gesetzestext, wenn einer angegeben ist.\n"
        # Ohne diese zwei Regeln wird aus einer Erklärung unbemerkt eine Auskunft: das Modell nennt
        # eine Zahl, der Nutzer trägt sie ein, und niemand hat je behauptet, sie sei geprüft.
        "Nenne NIE einen konkreten Betrag als den Wert des Nutzers — er trägt jeden Wert selbst ein "
        "und bestätigt ihn. Weißt du etwas nicht sicher, sag das ausdrücklich und setze "
        "\"unsicher\": true, statt zu raten.")
    return [{"role": "system", "content": system}, {"role": "user", "content": frage}]


def _llm_erklaerung(frage: str, kontext: str, user_id: str | None = None) -> dict:
    """{antwort, unsicher} — Fließtext-Antwort auf eine Nachfrage. Kein Feld, kein Event.

    PII-Filter wie im Vorschlags-Pfad, hier auf BEIDEN Teilen: die Nachfrage kommt vom Nutzer, und
    der Kontext trägt seine schon bestätigten Angaben. Audit nur Metadaten, nie den Text.

    Eine unbrauchbare Antwort (kaputtes JSON, leeres Feld) wird zu LlmNichtVerfuegbar — der
    Handler fällt dann auf dieselbe Erklär-Grenze zurück wie ohne Key. Ein halb geparster Satz
    wäre schlechter als ein ehrliches „nicht verfügbar"."""
    gefiltert, kategorien = filtere(frage)
    kontext_gefiltert, kategorien_k = filtere(kontext)
    import llm_client
    comp = llm_client.complete("chat", _erklaer_prompt(gefiltert, kontext_gefiltert),
                               schema=ERKLAER_SCHEMA)
    try:
        j = json.loads(comp.text)
        antwort = str(j["antwort"]).strip()
    except Exception as e:                                   # noqa: BLE001
        raise llm_client.LlmNichtVerfuegbar(
            f"Erklärung nicht verwertbar: {type(e).__name__}") from e
    if not antwort:
        raise llm_client.LlmNichtVerfuegbar("Erklärung war leer.")
    audit.append(user_id or "unbekannt", "llm_call", None,
                 f"task=erklaerung, pii_kategorien={kategorien}, kontext_kategorien={kategorien_k}, "
                 f"fragelaenge={len(frage)}, antwortlaenge={len(antwort)}, "
                 f"unsicher={bool(j.get('unsicher'))}")
    return {"antwort": antwort, "unsicher": bool(j.get("unsicher"))}


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
