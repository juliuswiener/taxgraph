"""Provider-agnostischer LLM-Client: EINE niedrig-level Wahrheit (`complete`) für jeden LLM-Call im Haut/K3-
Bereich. Task-spezifische Wrapper (Chat-Vorschläge, Kontoauszug-Klassifikation) leben NICHT hier, sondern in
der Handler-Schicht (api.py) bzw. beim jeweiligen Writer (kontoauszug_writer.llm_klassifikator_factory) — der
Client kennt keine Aufgaben, nur den rohen Call. stdlib-only (urllib), kein externes Paket.

⚠ CAP-GATED, $0 bis Julius den Key gibt: der echte LLM-Call passiert NUR wenn $LLM_API_KEY gesetzt ist — sonst
LlmNichtVerfuegbar → der Aufrufer fällt sauber auf seine Erklär-Grenze zurück (501/Vertrag bzw. stiller Skip),
nie ein Fake-Wert, nie ein Crash. KEIN Mock-Call (Julius-Regel: „nie Mock-LLM außer explizit verlangt" — Tests
injizieren eine Fixture-Antwort, rufen aber NIE echt).

PROVIDER-AGNOSTISCH: Endpunkt/Modell kommen aus der Umgebung ($LLM_API_BASE OpenAI-kompatibel, $LLM_MODEL), kein
Anbieter hartkodiert — Julius wählt Provider/Modell/Key. Der Schlüssel kommt AUSSCHLIESSLICH aus der Umgebung
(nie im Repo, nie geloggt)."""
from __future__ import annotations

import json
import os
import threading
import time
import urllib.error      # explizit: urllib.request zieht es zwar intern nach, aber darauf
import urllib.request    # zu bauen ist Zufall, kein Vertrag — und _call fängt HTTPError
from dataclasses import dataclass

_TIMEOUT = 30

# ------------------------------------------------- Obergrenze für die Erzeugung
# Die Anbieter-Doku zu DeepSeeks JSON-Ausgabe verlangt ausdrücklich ein gesetztes max_tokens,
# damit die JSON-Zeichenkette nicht mitten im Satz abbricht. Bisher stand hier gar keins — die
# Grenze war allein das 30-s-Zeitlimit, und ein durchdrehender Lauf wurde danach noch zweimal
# wiederholt.
#
# Die Zahl ist NICHT geraten, sondern über der Messung angesetzt (echter Aufruf 2026-08-21,
# Anbieter StreamLake): 1508 Antwort-Tokens, davon 1185 REASONING — der sichtbare Inhalt war
# knapp 1000 Zeichen. Genau das ist die Falle bei einem denkenden Modell: das Nachdenken zählt
# gegen dasselbe Budget wie die Antwort. Ein "vernünftig" klein gewähltes max_tokens erzeugt
# deshalb selbst den leeren Inhalt, den es verhindern soll. 8192 ist gut fünfmal der gemessene
# Bedarf — eine Deckelung gegen den Ausreisser, kein enger Rahmen.
_MAX_TOKENS = 8192

# ------------------------------------------------- Gründe, die der Aufrufer unterscheiden muss
# Kontrolliertes Vokabular aus UNSEREM Code, ausdrücklich kein Anbietertext: die Ausnahme trägt
# daneben eine gekürzte Provider-Meldung, und die darf nicht in ein Protokoll wandern, das nur
# Metadaten führen soll (produkt/store/audit.py). Ein Wort aus dieser Liste ist ein Metadatum.
GRUND_LEER = "leere_antwort"
GRUND_ABGESCHNITTEN = "abgeschnitten"

# Metadaten der zuletzt gelesenen Antwort (welcher Anbieter, wie die Erzeugung endete). Nötig,
# weil `_call` einen nackten `str` zurückgibt und diese Signatur bleiben muss — sie wird an
# mehreren Stellen als solche ersetzt. Thread-lokal statt Modul-global: der Server ist heute
# einfädig, aber eine Zuordnung, die nur unter dieser Bedingung stimmt, ist keine Zuordnung.
_letzte = threading.local()

# ------------------------------------------------- Wiederholung bei vorübergehenden Störungen
# (Audit res-product-clients-no-retry): jeder Fehlschlag wurde bisher sofort zu
# LlmNichtVerfuegbar — auch ein 503, das eine Sekunde später verschwunden wäre. Die
# Entwicklungs-Pipeline hat für dieselbe Anbieterklasse längst einen Backoff, und ihr eigener
# Kommentar hält fest, dass vorübergehende 503 dort gemessene Realität sind.
_VERSUCHE = 3                       # ein regulärer + zwei Wiederholungen
_BACKOFF_S = (1, 2)                 # wie pipeline/client.py: 2**versuch

# WELCHE Statuscodes wiederholt werden, ist NICHT aus der Pipeline übernommen — dort gilt 403
# als vorübergehend, ausdrücklich begründet mit einer OpenRouter-Eigenheit ("an invalid key
# gives 401, not 403"). Dieser Client ist provider-agnostisch, und sein eigener Kommentar unten
# hält fest, wofür 403 hier steht: "Budget limit exceeded". Ein erschöpftes Budget dreimal
# anzufragen kostet nur Zeit und ändert nichts — im einfädigen Server dreimal so lange
# Stillstand für dieselbe Antwort.
#
# Das ist der Punkt, an dem "die Pipeline macht es doch schon richtig" nicht trägt: die
# Erkenntnis ist anbieterspezifisch, die Übernahme wäre eine Verschlechterung gewesen.
_VORUEBERGEHEND = frozenset({429, 500, 502, 503, 504})


class LlmNichtVerfuegbar(Exception):
    """Kein $LLM_API_KEY gesetzt oder der Dienst antwortete nicht verwertbar — der /chat-Handler fällt auf die
    reine Erklär-Grenze zurück (die KI schlägt nichts vor, setzt erst recht nichts). Nie crashen, nie Fake."""


class _Voruebergehend(Exception):
    """Intern: dieser Fehlschlag ist einen weiteren Versuch wert. Bewusst NICHT von
    LlmNichtVerfuegbar abgeleitet — sonst fingen ihn die Aufrufer, die auf ihre Erklär-Grenze
    zurückfallen, schon ab, bevor überhaupt wiederholt wurde. Verlässt dieses Modul nie."""


def _mit_grund(e: Exception, grund: str) -> Exception:
    """Hängt der Ausnahme ein Wort aus dem kontrollierten Vokabular an. `getattr(e, "grund", "")`
    beim Aufrufer — so bleibt `except LlmNichtVerfuegbar` überall unverändert gültig."""
    e.grund = grund
    return e


def _merke(provider: str, ende: str) -> None:
    _letzte.meta = {"provider": provider, "finish_reason": ende}


def letzte_meta() -> dict:
    """Metadaten der letzten Antwort in DIESEM Thread: `provider` (wer bei einem Vermittler wie
    OpenRouter tatsächlich geantwortet hat) und `finish_reason`. Wird zu Beginn jedes `_call`
    geleert, kann also nie die Angabe eines früheren Aufrufs zurückgeben. Leeres dict, solange
    keine Antwort gelesen wurde.

    Warum das überhaupt herausgeführt wird: ohne den Namen des antwortenden Endpunkts ist nicht
    nachprüfbar, ob die Absicherung unten (`require_parameters`) greift — und eine Absicherung,
    deren Wirkung man nicht messen kann, ist nur ein Vorsatz."""
    return dict(getattr(_letzte, "meta", None) or {})


def _key() -> str:
    k = os.environ.get("LLM_API_KEY", "").strip()
    if not k:
        raise LlmNichtVerfuegbar("kein LLM_API_KEY in der Umgebung — Chat bleibt reine Erklär-Grenze ($0).")
    return k


def _base() -> str:
    return os.environ.get("LLM_API_BASE", "").strip().rstrip("/")


def _model() -> str:
    return os.environ.get("LLM_MODEL", "").strip()


def _call(messages: list[dict], schema: dict | None = None) -> str:
    """OpenAI-kompatibler Chat-Completions-Call (provider-agnostisch). Roher Antwort-Text. Jeder Fehler
    (kein Base/Modell, Netz, HTTP, JSON) → LlmNichtVerfuegbar (Aufrufer fällt auf die Erklär-Grenze zurück).

    `schema`: optionales JSON-Schema (Form {"name": …, "strict": True, "schema": {…}}). Ist es gesetzt,
    geht response_format={"type":"json_schema", …} raus statt des schwächeren json_object — der Provider
    erzwingt dann die Struktur, statt sie nur zu erbitten. Ohne Schema bleibt es beim Objekt-Modus.

    DASS DAS SCHEMA ERZWUNGEN WIRD, IST NICHT UMSONST ZU HABEN. OpenRouter kann dieselbe Modell-Kennung
    über viele Endpunkte bedienen, und die Fähigkeit hängt am ENDPUNKT, nicht am Modell: "Support for
    structured outputs is determined per endpoint rather than solely per model." Unter der Standard-
    Wegewahl bekommen Endpunkte, die einen Parameter nicht können, die Anfrage trotzdem — sie ignorieren
    ihn dann stillschweigend. Gemessen 2026-08-21 an deepseek/deepseek-v4-pro: 19 Endpunkte, 7 davon
    OHNE `structured_outputs` (darunter DeepSeek selbst, dessen Chat-API nur `json_object` kennt; BaseTen
    führt nicht einmal `response_format`). Ohne die Zeile unten war also gut jeder dritte Endpunkt einer,
    bei dem wir irgendein JSON zurückbekommen und es für schema-geprüft halten — fail-open.
    `provider.require_parameters` schliesst genau diese Endpunkte aus. NUR mit Schema: ohne eines gäbe es
    nichts zu erzwingen, und die Wegewahl würde grundlos verengt (der Kontoauszug-Pfad ruft ohne Schema).

    Nachprüfbar ist das über `letzte_meta()["provider"]` — wer tatsächlich geantwortet hat."""
    base, model = _base(), _model()
    if not base or not model:
        raise LlmNichtVerfuegbar("LLM_API_BASE/LLM_MODEL nicht gesetzt — Provider nicht konfiguriert.")
    schluessel = _key()
    format_teil = ({"type": "json_schema", "json_schema": schema} if schema
                   else {"type": "json_object"})
    nutzlast = {"model": model, "messages": messages, "temperature": 0,
                "response_format": format_teil, "max_tokens": _MAX_TOKENS}
    if schema:
        nutzlast["provider"] = {"require_parameters": True}
    body = json.dumps(nutzlast).encode("utf-8")
    req = urllib.request.Request(
        f"{base}/chat/completions", data=body, method="POST",
        headers={"Authorization": f"Bearer {schluessel}", "Content-Type": "application/json"})
    _merke("", "")            # nie die Angabe eines früheren Aufrufs stehen lassen
    for versuch in range(_VERSUCHE):
        letzter = versuch == _VERSUCHE - 1
        try:
            return _ein_versuch(req, schluessel)
        except _Voruebergehend as e:
            if letzter:
                raise _mit_grund(
                    LlmNichtVerfuegbar(f"LLM-Aufruf nach {_VERSUCHE} Versuchen fehlgeschlagen: {e}"),
                    getattr(e, "grund", "")) from e
            time.sleep(_BACKOFF_S[versuch])
    raise AssertionError("unerreichbar")   # die Schleife kehrt zurück oder wirft


def _ein_versuch(req, schluessel: str) -> str:
    """Ein einzelner Aufruf. Wirft _Voruebergehend bei Störungen, die vorübergehen können, und
    LlmNichtVerfuegbar bei allem, was ein zweiter Versuch nicht heilt."""
    try:
        with urllib.request.urlopen(req, timeout=_TIMEOUT) as r:
            j = json.loads(r.read())
    except urllib.error.HTTPError as e:
        # Statuscode UND Provider-Meldung mitnehmen (gekürzt). Vorher stand hier nur
        # type(e).__name__, also "HTTPError" — und damit sah eine erschöpfte Budget-Grenze
        # (403 "Budget limit exceeded") exakt aus wie ein falscher Key, ein totes Modell oder
        # ein Tippfehler in der Base-URL. Gemessen 2026-08-14: die Ursache war nur über einen
        # Direktaufruf am Client vorbei zu finden.
        # KEIN Leak-Risiko: der Key steht im REQUEST-Header, nicht in der Antwort, und diese
        # Meldung erreicht den Nutzer ohnehin nie — der /chat-Handler antwortet mit der festen
        # CHAT_501-Konstante. Sie ist für das Server-Log und die Diagnose da.
        try:
            detail = e.read().decode("utf-8", "replace")[:300]
        except Exception:                                    # noqa: BLE001
            detail = ""
        # Der Schlüssel steht im Request-Header — manche Provider spiegeln ihn in der
        # Fehlerantwort zurück ("invalid key sk-…"). Ohne diese Maskierung landete er über die
        # Ausnahme im Log. Gefunden vom eigenen Test dieser Änderung, nicht im Betrieb.
        if schluessel and schluessel in detail:
            detail = detail.replace(schluessel, "<KEY>")
        if e.code in _VORUEBERGEHEND:
            raise _Voruebergehend(f"HTTP {e.code} {detail}") from e
        raise LlmNichtVerfuegbar(f"LLM-Aufruf fehlgeschlagen: HTTP {e.code} {detail}") from e
    except urllib.error.URLError as e:
        # Netzstörung oder Zeitüberschreitung (socket.timeout kommt hier als URLError.reason an)
        # — beides kann vorübergehen, anders als eine falsche Base-URL. Die Unterscheidung ist
        # aus der Ausnahme allein nicht zu treffen; ein zweiter Versuch kostet im schlimmsten
        # Fall eine Sekunde und den zweiten Fehlschlag.
        raise _Voruebergehend(f"{type(e).__name__}: {e.reason}") from e
    except TimeoutError as e:
        # Der Typname gehört MIT in die Meldung, nicht nur die verständliche Beschreibung: die
        # Diagnose-Lehre von 2026-08-14 war, dass eine Meldung ohne Ursache jede Fehlerart gleich
        # aussehen lässt (test_llm_client_fehlerdiagnose.py hält das fest). Der eigene Umbau
        # hätte das beinahe wieder weggenommen — aufgefallen ist es nur, weil jener Test rot wurde.
        raise _Voruebergehend(f"{type(e).__name__}: Zeitüberschreitung nach {_TIMEOUT}s") from e
    except Exception as e:
        # Alles Übrige (kaputtes JSON) heilt kein zweiter Versuch.
        raise LlmNichtVerfuegbar(f"LLM-Aufruf fehlgeschlagen: {type(e).__name__}") from e
    return _inhalt(j)


def _inhalt(j: dict) -> str:
    """Antwort-JSON → Inhalt, und die beiden Fälle benannt, die bisher als leerer Text durchliefen.

    AUSSERHALB des try-Blocks oben, und das ist der Punkt: dort fängt ein `except Exception` alles
    ein und macht LlmNichtVerfuegbar daraus — ein hier geworfenes `_Voruebergehend` würde also
    verschluckt, und die Wiederholung fände nie statt. Ein Gate, das im Aufräum-Zweig eines anderen
    hängt, ist keins."""
    try:
        wahl = j["choices"][0]
        inhalt = wahl["message"].get("content")
        ende = str(wahl.get("finish_reason") or "")
    except Exception as e:                                   # noqa: BLE001
        # Fehlende choices-Struktur — wie bisher: kein zweiter Versuch.
        raise LlmNichtVerfuegbar(f"LLM-Aufruf fehlgeschlagen: {type(e).__name__}") from e
    _merke(str(j.get("provider") or ""), ende)
    # REIHENFOLGE IST DIE AUSSAGE. Ein denkendes Modell kann sein ganzes Token-Budget im
    # Nachdenken verbrauchen; dann ist der Inhalt leer UND abgeschnitten. "Abgeschnitten" ist
    # die genauere Diagnose und die einzige der beiden, die ein zweiter Versuch nicht heilt:
    # bei temperature=0 läuft derselbe Aufruf in dieselbe Grenze, dreimal für dieselbe Antwort.
    if ende == "length":
        raise _mit_grund(LlmNichtVerfuegbar(
            f"LLM-Antwort bei {_MAX_TOKENS} Tokens abgeschnitten — unvollständiges JSON."),
            GRUND_ABGESCHNITTEN)
    if not (inhalt or "").strip():
        # Kein Ausrutscher, sondern ein vom Anbieter selbst benannter Fall: die DeepSeek-Doku zur
        # JSON-Ausgabe hält fest, die API gebe gelegentlich leeren Inhalt zurück, daran werde
        # gearbeitet. Damit ist er per Definition einen weiteren Versuch wert — dieselbe Bauart
        # wie ein 503, und die Wiederholschleife dafür steht schon. Bisher lief er als leerer
        # String bis in `_chat_parse`/`_antwort_parse` durch und endete dort als "keine
        # Vorschläge, keine Antwort": nicht von einer Nachricht zu unterscheiden, zu der das
        # Modell schlicht nichts zu sagen hatte.
        raise _mit_grund(_Voruebergehend("leerer Inhalt vom Anbieter"), GRUND_LEER)
    return inhalt


@dataclass
class Completion:
    """Rohe LLM-Antwort: `text` plus die Metadaten der Antwort — `provider` (bei einem Vermittler
    wie OpenRouter der Endpunkt, der TATSÄCHLICH geantwortet hat) und `finish` (finish_reason).
    Beides ist Metadatum, nie Inhalt; Vorgabe leer, damit Fixture-Antworten in Tests unverändert
    gebaut werden können. Der Client kennt weiter keine Aufgabe, nur den Call."""
    text: str
    provider: str = ""
    finish: str = ""


def complete(role: str, messages: list[dict], fixture_id: str | None = None,
             schema: dict | None = None) -> Completion:
    """Die EINE niedrig-level Wahrheit: OpenAI-kompatibler Chat-Call → Completion. Cap-gated wie `_call` (kein
    Key/Base/Modell → LlmNichtVerfuegbar, kein Netz-Zugriff). `role`/`fixture_id` sind Interface-Parität zum
    pipeline-Client (kontoauszug_writer.llm_klassifikator_factory erwartet `client.complete(role, msgs,
    fixture_id=)`) — dieser Haut-Client hat EIN Modell aus der Umgebung, kein Rollen-Routing/Fixture-Replay
    (das lebt in pipeline/client.py, eine andere Baustelle).

    `schema`: optionales JSON-Schema, das der Provider erzwingt (s. _call). Der Kontoauszug-Pfad
    ruft weiter ohne — dessen Fixture-Replay-Signatur bleibt unberührt."""
    text = _call(messages, schema=schema)
    meta = letzte_meta()
    return Completion(text=text, provider=meta.get("provider", ""),
                      finish=meta.get("finish_reason", ""))
