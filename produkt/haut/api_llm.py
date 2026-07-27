"""LLM-Integration für Chat-Berater + Kontoauszug-Klassifikation (Bright-Line Isolation).

Dieses Modul ISOLIERT den llm_client-Import auf eine Datei (hier). api.py + Server
importieren diese Funktionen, ohne selbst llm_client zu kennen. Lazy imports innerhalb
der Funktionen halten die Abhängigkeit locker. Exportiert LlmNichtVerfuegbar damit
api.py sie in except-Clauses nutzen kann."""

import json
import kontoauszug_writer as KW

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
        for f in katalog)
    system = (
        "Du bist ein Steuer-Assistent, der aus der Freitext-Beschreibung eines Nutzers Feld-Werte VORSCHLÄGT. "
        "Du SETZT nie einen Wert und triffst keine rechtliche Entscheidung — der Mensch bestätigt jeden Vorschlag. "
        "Du darfst NUR diese Felder vorschlagen (keine anderen):\n" + felder + "\n\n"
        "Geld-Beträge MUSST du als GANZZAHL in CENT angeben (EUR × 100), z.B. 2156,50 € → 215650. "
        "Niemals als EUR-Kommazahl oder EUR-Ganzzahl.\n"
        "Antworte AUSSCHLIESSLICH mit einem JSON-Array [{\"feld_id\":\"…\",\"wert\":…,\"begruendung\":\"kurz\"}], "
        "nur Felder für die die Beschreibung einen konkreten Wert hergibt, sonst []. Kein Fließtext.")
    return [{"role": "system", "content": system}, {"role": "user", "content": freitext}]


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
                        "begruendung": str(v.get("begruendung", ""))[:200]})
    return out


def _llm_vorschlaege(freitext: str, katalog: list[dict]) -> list[dict]:
    """Chat-Task-Wrapper (Handler-Schicht) ÜBER llm_client.complete (der einen niedrig-level Wahrheit). Cap-
    gated: kein Key/Base/Modell → LlmNichtVerfuegbar propagiert (der /chat-Handler fängt sie → 501). Der
    Aufrufer schreibt jeden Vorschlag als VORLÄUFIGES Event (Store-Auflage A + Katalog-Check erzwingen die
    Sicherheit); der Mensch bestätigt via Hold-Confirm."""
    if not (freitext or "").strip():
        return []
    import llm_client
    comp = llm_client.complete("chat", _chat_prompt(freitext, katalog))
    return _chat_parse(comp.text)


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
