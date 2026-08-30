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

# ------------------------------------------------- Wanduhr-Grenze für EINEN Aufruf
# `_TIMEOUT` IST KEINE WANDUHR-GRENZE, und das war bis 2026-08-28 niemandem klar. Es geht als
# Socket-Timeout an `urlopen` und gilt je LESEOPERATION — eine Antwort, die tröpfelt, setzt es
# bei jedem Paket zurück und reisst es nie. Gemessen an 57 echten Stufen-Aufrufen: der Median
# liegt bei 55 s, der längste erfolgreiche bei 128,7 s, und ein einzelner Aufruf lief 187,8 s.
# Alle unter einem „30-s-Timeout". Es gab damit KEINE Obergrenze dafür, wie lange ein Nutzer auf
# eine Chat-Antwort wartet; 600 s hätte genauso wenig etwas verhindert.
#
# DIE ZAHL IST GEMESSEN, NICHT GEWÄHLT. An denselben 57 Aufrufen, gezählt wurde, wie viele HEUTE
# ERFOLGREICHE Aufrufe eine Grenze abschneiden würde:
#     90 s → 8 von 56 (14,3 %)   120 s → 2 (3,6 %)   150 s → 0   180 s → 0
# 150 s ist die kleinste runde Zahl, die keinen einzigen gemessenen Erfolg kostet, mit 21 s
# Luft über dem längsten (128,7 s). Die 90 s aus der Entscheidung sind das Budget für die
# WIEDERHOLUNG, nicht für den ersten Versuch — als Grenze für jeden Aufruf hätten sie jeden
# siebten funktionierenden Aufruf abgeschnitten, um einen von 29 zu retten.
#
# EHRLICHE GRENZE DER GRENZE: geprüft wird ZWISCHEN den Leseoperationen. Eine einzelne blockierte
# Leseoperation läuft weiter bis `_TIMEOUT`, im schlechtesten Fall überschreitet der Aufruf die
# Frist also um bis zu 30 s. Exakter ginge es nur mit einem anderen HTTP-Client oder einem
# Wächter-Thread — beides ein eigener Umbau, und für eine Obergrenze, die es vorher gar nicht
# gab, ist „150 bis 180 statt unbegrenzt" die richtige erste Stufe.
_FRIST_S = 150

# Budget für den zweiten Versuch nach einer abgeschnittenen Antwort (Entscheidung team-lead,
# 2026-08-28). Kleiner als `_FRIST_S`, und das ist der Sinn: der Median-Aufruf braucht 55 s, 90 s
# decken rund neun von zehn. Wer auch beim zweiten Mal länger braucht, ist wieder auf dem Weg in
# die Token-Grenze — und dann ist Abbrechen billiger als Warten.
_FRIST_WIEDERHOLUNG_S = 90

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
GRUND_FRIST = "frist_ueberschritten"

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


class _Abgeschnitten(_Voruebergehend):
    """Die Antwort lief in die Token-Grenze — einen zweiten Versuch wert, aber nur EINEN.

    BIS 2026-08-28 STAND HIER DAS GEGENTEIL, und die Begründung war falsch: „bei temperature=0
    läuft derselbe Aufruf in dieselbe Grenze, dreimal für dieselbe Antwort." Das klingt zwingend
    und ist an unseren eigenen Daten widerlegt:

      * Derselbe Nutzertext (137 Zeichen) lief 17 Minuten VOR dem Ausfall durch — gleiche Eingabe,
        gleicher Code, einmal rot, einmal grün.
      * Bei identischer Eingabe ordnet Stufe 2 mal 5, mal 60 Regeln zu (Faktor 11).
      * api_llm.py hält dieselbe Nicht-Determiniertheit seit dem 2026-08-14 fest: „das Modell
        antwortet trotz temperature=0 NICHT deterministisch (derselbe Satz, derselbe Code: einmal
        4, einmal 3 Vorschläge)."

    `temperature=0` heisst nicht deterministisch — es heisst nur „nimm das wahrscheinlichste
    Token". Batching, Fliesskomma-Reihenfolge und wechselnde Endpunkte hinter einem Vermittler
    reichen für einen anderen Lauf. Ein denkendes Modell grübelt deshalb bei derselben Frage
    einmal 200 und einmal 8.000 Tokens, und nur im zweiten Fall reisst die Grenze.

    NUR EIN ZWEITER VERSUCH, nicht die drei der übrigen vorübergehenden Fehler: ein abgeschnittener
    Aufruf hat bis zur Token-Grenze erzeugt und ist damit per Konstruktion der LANGSAMSTE, den es
    gibt (gemessen 187,8 s — länger als jeder erfolgreiche). Jede Wiederholung startet aus dem
    schlechtesten Fall; drei davon wären für den Nutzer ein leerer Bildschirm über zehn Minuten."""


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
    # Abgeschnittene Antworten werden GETRENNT gezählt: sie sind teuer (s. _Abgeschnitten) und
    # bekommen deshalb genau einen zweiten Versuch, während ein 503 weiter drei bekommt.
    abgeschnitten = 0
    for versuch in range(_VERSUCHE):
        letzter = versuch == _VERSUCHE - 1
        try:
            # Der zweite Versuch nach einem Abschnitt bekommt das kleinere Budget.
            return _ein_versuch(req, schluessel,
                                _FRIST_WIEDERHOLUNG_S if abgeschnitten else _FRIST_S)
        except _Abgeschnitten as e:
            abgeschnitten += 1
            if abgeschnitten >= _ABGESCHNITTEN_MAX or letzter:
                raise _aufgegeben(e, abgeschnitten)
            time.sleep(_BACKOFF_S[versuch])
        except _Voruebergehend as e:
            if letzter:
                raise _aufgegeben(e, _VERSUCHE)
            time.sleep(_BACKOFF_S[versuch])
    raise AssertionError("unerreichbar")   # die Schleife kehrt zurück oder wirft


# Ein regulärer Versuch + EINE Wiederholung. Bewusst nicht `_VERSUCHE`: s. _Abgeschnitten.
_ABGESCHNITTEN_MAX = 2


def _aufgegeben(e: Exception, versuche: int) -> LlmNichtVerfuegbar:
    """Endgültiger Fehlschlag, mit der Zahl der Versuche daran.

    Die Zahl wandert über `getattr(e, "versuche", 1)` bis in den Fluss-Mitschnitt (api_llm
    `gescheitert`). Auflage team-lead 2026-08-28, und sie ist der Punkt: eine Wiederholung, die
    im Protokoll nicht auftaucht, VERSTECKT den Ausfall — hinterher sähe ein Aufruf, der zweimal
    scheiterte, aus wie einer, der es einmal versucht hat. Es ist eine Metadaten-Zahl aus unserem
    eigenen Code, kein Anbietertext; sie darf ins Protokoll."""
    neu = LlmNichtVerfuegbar(f"LLM-Aufruf nach {versuche} Versuch(en) fehlgeschlagen: {e}")
    neu.versuche = versuche
    return _mit_grund(neu, getattr(e, "grund", ""))


def _lies_bis(r, ende: float) -> bytes:
    """Die Antwort lesen, aber nicht über `ende` (monotone Uhr) hinaus.

    `r.read()` in einem Zug hat keine Obergrenze: es kehrt zurück, wenn der Anbieter fertig ist,
    und der Socket-Timeout beginnt bei jedem Paket von vorn. Genau daran lief ein Aufruf 187,8 s
    unter einem „30-s-Timeout" (s. `_FRIST_S`). In Stücken gelesen, lässt sich zwischen den
    Paketen auf die Uhr sehen.

    Die monotone Uhr, nicht die Wanduhr: eine Zeitumstellung oder ein NTP-Sprung darf eine
    laufende Anfrage weder verlängern noch abwürgen.

    KEIN zweiter Versuch bei Fristablauf, anders als bei 503 oder leerer Antwort: gemessen
    überschreitet KEIN einziger erfolgreicher Aufruf die Frist (0 von 56, längster 128,7 s). Wer
    sie reisst, ist kein Ausrutscher, sondern auf dem Weg in die Token-Grenze — und drei Versuche
    à 150 s wären 450 s Warten, also genau der unbegrenzte Bildschirm, gegen den die Frist steht.
    Deshalb LlmNichtVerfuegbar und nicht _Voruebergehend."""
    stuecke = []
    while True:
        if time.monotonic() >= ende:
            raise _mit_grund(LlmNichtVerfuegbar("LLM-Antwort überschritt die Frist"), GRUND_FRIST)
        stueck = r.read(65536)
        if not stueck:
            return b"".join(stuecke)
        stuecke.append(stueck)


def _ein_versuch(req, schluessel: str, frist_s: float = _FRIST_S) -> str:
    """Ein einzelner Aufruf. Wirft _Voruebergehend bei Störungen, die vorübergehen können, und
    LlmNichtVerfuegbar bei allem, was ein zweiter Versuch nicht heilt.

    `frist_s` ist die Wanduhr-Grenze für DIESEN Versuch — sie deckt Verbindungsaufbau und Lesen
    zusammen, denn beide kosten den Nutzer dieselbe Wartezeit."""
    ende = time.monotonic() + frist_s
    try:
        # Der Socket-Timeout bleibt die Grenze für EINE Leseoperation; die Frist deckelt den
        # Aufruf als Ganzes. `min` verhindert, dass ein einzelner Lesevorgang länger blockiert,
        # als die Frist überhaupt noch hergibt.
        with urllib.request.urlopen(req, timeout=min(_TIMEOUT, frist_s)) as r:
            j = json.loads(_lies_bis(r, ende))
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
    except (_Voruebergehend, LlmNichtVerfuegbar):
        # AUS `_lies_bis`, schon eingeordnet und mit Grund versehen. Ohne diese Zeile finge der
        # Auffang-Zweig darunter sie ein und machte daraus einen namenlosen Fehler ohne `grund` —
        # dieselbe Falle, der `_inhalt()` unten dadurch entgeht, dass es AUSSERHALB des try steht.
        # Ein Gate, das im Aufräum-Zweig eines anderen hängt, ist keins.
        raise
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
    # Nachdenken verbrauchen; dann ist der Inhalt leer UND abgeschnitten. "Abgeschnitten" ist die
    # genauere Diagnose von beiden.
    #
    # BIS 2026-08-28 STAND HIER, ein zweiter Versuch heile das nicht — "bei temperature=0 läuft
    # derselbe Aufruf in dieselbe Grenze, dreimal für dieselbe Antwort". Das ist gemessen falsch,
    # und es war die BEGRÜNDUNG einer Entscheidung, nicht bloss eine Notiz: derselbe Nutzertext
    # lief 17 Minuten vor dem Ausfall durch, und dieselbe Eingabe erzeugt mal 5, mal 60
    # Regel-Zuordnungen. Die volle Beweisführung steht bei `_Abgeschnitten`. Ein Versuch ist es
    # deshalb wert — genau EINER, denn er ist teuer.
    if ende == "length":
        raise _mit_grund(_Abgeschnitten(
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
