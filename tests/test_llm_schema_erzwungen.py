"""Wir GLAUBTEN, ein JSON-Schema zu erzwingen — eingeschaltet war die Absicherung dafür nicht.

BEFUND (2026-08-21). `llm_client._call` sendet `response_format={"type":"json_schema", …}` mit
`strict: True`. Bei OpenRouter hängt die Fähigkeit dazu aber am ENDPUNKT, nicht am Modell, und
unter der Standard-Wegewahl bekommen Endpunkte, die einen Parameter nicht können, die Anfrage
trotzdem — sie ignorieren ihn stillschweigend. Am selben Tag über die freie Endpunkt-Liste
gemessen: deepseek/deepseek-v4-pro wird von 19 Endpunkten bedient, 7 davon führen
`structured_outputs` NICHT (darunter DeepSeek selbst; BaseTen nicht einmal `response_format`).
Gut jeder dritte Weg war also einer, auf dem wir irgendein JSON zurückbekommen und es für
schema-geprüft halten. Fail-open, und zwar eines, das nie auffällt.

Der Code-Kommentar sagte „deepseek/deepseek-v4-pro kann es, gemessen 2026-08-14". Das stimmte
sogar — es war nur keine Aussage über den nächsten Aufruf, weil die Fähigkeit am Endpunkt hängt
und die Wegewahl bei jedem Aufruf neu fällt.

WAS DIESE DATEI PRÜFT, ist deshalb nicht „das Schema wirkt" (das kann kein Test hier
feststellen, das entscheidet der Anbieter), sondern die drei Dinge, die WIR in der Hand haben:
  1. `require_parameters` geht mit, wenn ein Schema gesetzt ist — und nur dann.
  2. Der antwortende Anbieter wird sichtbar. Ohne ihn wäre Punkt 1 nicht nachprüfbar, und eine
     Absicherung, deren Wirkung niemand messen kann, ist nur ein Vorsatz.
  3. Eine leere oder abgeschnittene Antwort hat einen Namen, statt als „keine Vorschläge, keine
     Antwort" durchzulaufen — ununterscheidbar von einer Nachricht, zu der das Modell nichts zu
     sagen hatte. Der Anbieter nennt die leere Antwort in seiner eigenen Doku als bekanntes
     Problem; ein erwarteter Fall gehört benannt, nicht überrascht entdeckt.

NULL LLM: kein echter Aufruf. `urlopen` wird ersetzt — eine Ebene TIEFER als die üblichen
`complete`-Stubs, denn nur so ist der gesendete Body sichtbar, und um den geht es hier.
"""
from __future__ import annotations

import json
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/import", "produkt/traverser"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import api_llm                    # noqa: E402
import audit as AUDIT             # noqa: E402
import llm_client as LC           # noqa: E402

KATALOG = [{"feld_id": "bruttoarbeitslohn", "fragetext_laie": "Bruttoarbeitslohn?", "typ": "EURO"}]


@pytest.fixture
def konfiguriert(monkeypatch):
    """Key/Base/Modell gesetzt, damit das Cap-Gate den Request überhaupt bauen lässt. Der Wert
    ist erfunden — es geht nie ein Byte ins Netz, `urlopen` ist in jedem Test ersetzt."""
    monkeypatch.setenv("LLM_API_KEY", "test-schluessel-nicht-echt")
    monkeypatch.setenv("LLM_API_BASE", "https://example.test/v1")
    monkeypatch.setenv("LLM_MODEL", "test/modell")
    monkeypatch.setattr(LC.time, "sleep", lambda s: None)     # keine echten Wartezeiten


class _Antwort:
    """Minimaler urlopen-Ersatz. KEIN Mock eines Anbieters: liefert genau die Hülle, die der
    Client liest (choices/message/content, finish_reason, provider) — nichts Inhaltliches."""

    def __init__(self, roh: bytes):
        self._roh = roh

    # `read(n)` wie ein echter HTTPResponse, und beim zweiten Mal leer: der Client liest seit
    # 2026-08-28 in Stücken, um zwischendurch auf die Uhr zu sehen (llm_client `_lies_bis`).
    # Eine Attrappe, die immer alles zurückgibt, käme aus dieser Schleife nie heraus.
    def read(self, n: int = -1) -> bytes:
        roh, self._roh = self._roh, b""
        return roh

    def __enter__(self):
        return self

    def __exit__(self, *a):
        return False


def _hülle(inhalt, provider="TestAnbieter", finish="stop") -> bytes:
    return json.dumps({
        "provider": provider,
        "choices": [{"finish_reason": finish, "message": {"content": inhalt}}],
    }).encode()


def _dialog_json(antwort="", vorschlaege=()) -> str:
    return json.dumps({"vorschlaege": list(vorschlaege), "antwort": antwort, "unsicher": False})


def _aufzeichnend(monkeypatch, folge):
    """Ersetzt urlopen durch eine Attrappe, die die gesendeten Bodys sammelt und der Reihe nach
    aus `folge` antwortet (das letzte Element wiederholt sich). Gibt die Body-Liste zurück."""
    gesendet, zaehler = [], []

    def _urlopen(req, timeout=None):
        gesendet.append(json.loads(req.data.decode()))
        i = min(len(zaehler), len(folge) - 1)
        zaehler.append(1)
        return _Antwort(folge[i])

    monkeypatch.setattr(LC.urllib.request, "urlopen", _urlopen)
    return gesendet


# ============================================================ 1. require_parameters am Draht

def test_mit_schema_geht_require_parameters_mit(konfiguriert, monkeypatch):
    """Der Kern des Befundes. Ohne diese Zeile darf OpenRouter an einen Endpunkt weiterleiten,
    der `json_schema` stillschweigend ignoriert — 7 von 19 gemessenen Endpunkten dieses Modells
    können es nicht."""
    gesendet = _aufzeichnend(monkeypatch, [_hülle(_dialog_json())])
    LC._call([{"role": "user", "content": "x"}], schema=api_llm.DIALOG_SCHEMA)

    body = gesendet[0]
    assert body.get("provider", {}).get("require_parameters") is True, (
        "Ein Schema geht raus, aber ohne provider.require_parameters — der Vermittler darf die "
        "Anfrage an einen Endpunkt geben, der das Schema ignoriert, und wir halten irgendein "
        f"JSON für schema-geprüft.\nGesendet: {json.dumps(body.get('provider'))}")
    assert body["response_format"]["type"] == "json_schema", (
        "Das Schema selbst geht gar nicht mehr raus — dann sichert require_parameters nichts ab.")


def test_ohne_schema_kein_provider_block(konfiguriert, monkeypatch):
    """Die andere Hälfte, und keine Formalie: `require_parameters` schliesst Endpunkte aus. Ohne
    Schema gäbe es nichts zu erzwingen, und die Verengung wäre grundlos — der Kontoauszug-Pfad
    ruft ohne Schema und würde für nichts Anbieter verlieren."""
    gesendet = _aufzeichnend(monkeypatch, [_hülle('{"kategorie": "spende"}')])
    LC._call([{"role": "user", "content": "x"}])

    body = gesendet[0]
    assert "provider" not in body, (
        f"Ohne Schema geht trotzdem ein provider-Block raus: {json.dumps(body.get('provider'))} — "
        "das verengt die Wegewahl, ohne dass es etwas abzusichern gäbe.")
    assert body["response_format"]["type"] == "json_object"


def test_max_tokens_geht_mit(konfiguriert, monkeypatch):
    """Die Anbieter-Doku verlangt ein gesetztes max_tokens, damit das JSON nicht mitten im Satz
    abbricht. Bisher stand keins im Body — die einzige Grenze war das Zeitlimit, und danach
    wurde noch zweimal wiederholt."""
    gesendet = _aufzeichnend(monkeypatch, [_hülle(_dialog_json())])
    LC._call([{"role": "user", "content": "x"}], schema=api_llm.DIALOG_SCHEMA)
    assert gesendet[0].get("max_tokens") == LC._MAX_TOKENS, (
        f"max_tokens fehlt im Body: {gesendet[0].get('max_tokens')!r}")


# ============================================================ 2. Wer hat geantwortet?

def _audit_zeilen(tmp_path) -> list[str]:
    pfad = os.path.join(str(tmp_path), "audit.jsonl")
    if not os.path.exists(pfad):
        return []
    return [json.loads(z)["detail"] or "" for z in open(pfad, encoding="utf-8") if z.strip()]


def test_anbietername_landet_im_audit(konfiguriert, monkeypatch, tmp_path):
    """Ohne diese Angabe ist `require_parameters` nicht nachprüfbar: man könnte nie feststellen,
    ob die Absicherung greift oder ob wir gerade auf einem Endpunkt landen, der das Schema
    wegwirft."""
    monkeypatch.setattr(AUDIT, "AUDIT_DIR", str(tmp_path))
    _aufzeichnend(monkeypatch, [_hülle(_dialog_json(antwort="Kurz erklärt."),
                                       provider="StreamLake")])
    api_llm._llm_dialog("Ich habe 62000 Euro verdient.", KATALOG, user_id="prüfer")

    zeilen = _audit_zeilen(tmp_path)
    assert zeilen, "kein Audit-Eintrag geschrieben"
    assert "StreamLake" in zeilen[-1], (
        f"Der antwortende Anbieter steht nicht im Audit-Eintrag: {zeilen[-1]!r}")


def test_antworttext_landet_nie_im_audit(konfiguriert, monkeypatch, tmp_path):
    """Die Grenze, die der Anbietername nicht verschieben darf. Das Protokoll führt
    AUSSCHLIESSLICH Metadaten (produkt/store/audit.py) — ein Anbietername ist eines, der
    Antworttext und jedes Zitat daraus ist keines."""
    monkeypatch.setattr(AUDIT, "AUDIT_DIR", str(tmp_path))
    geheim = "UNVERWECHSELBARER-ANTWORTTEXT-4711"
    beleg = "62000 Euro"
    _aufzeichnend(monkeypatch, [_hülle(_dialog_json(
        antwort=geheim,
        vorschlaege=[{"feld_id": "bruttoarbeitslohn", "wert": 6200000, "beleg": beleg,
                      "begruendung": geheim}]))])
    erg = api_llm._llm_dialog("Ich habe 62000 Euro verdient.", KATALOG, user_id="prüfer")
    assert erg["antwort"] == geheim and len(erg["vorschlaege"]) == 1, (
        "Vorbedingung verfehlt — der Text kam gar nicht erst durch, der Test prüfte nichts.")

    zeilen = _audit_zeilen(tmp_path)
    assert geheim not in "\n".join(zeilen), (
        f"Antworttext im Audit-Protokoll: {zeilen[-1]!r}")
    assert beleg not in "\n".join(zeilen), (
        f"Zitat aus der Nutzereingabe im Audit-Protokoll: {zeilen[-1]!r}")


# ============================================================ 3. Leer und abgeschnitten

def test_leere_antwort_wird_wiederholt(konfiguriert, monkeypatch):
    """Der Anbieter nennt die leere Antwort in seiner eigenen Doku als bekanntes, in Arbeit
    befindliches Problem. Damit ist sie per Definition einen weiteren Versuch wert — dieselbe
    Bauart wie ein 503, und die Wiederholschleife dafür steht längst."""
    gesendet = _aufzeichnend(monkeypatch, [_hülle(""), _hülle(_dialog_json(antwort="Da bin ich."))])
    text = LC._call([{"role": "user", "content": "x"}], schema=api_llm.DIALOG_SCHEMA)

    assert json.loads(text)["antwort"] == "Da bin ich."
    assert len(gesendet) == 2, (
        f"{len(gesendet)} Versuche — eine leere Antwort wurde nicht wiederholt, obwohl der "
        f"Anbieter sie selbst als vorübergehend beschreibt.")


def test_dauerhaft_leere_antwort_endet_benannt_statt_still(konfiguriert, monkeypatch):
    """DER EIGENTLICHE PUNKT. Vorher lief ein leerer Inhalt als leerer String bis in
    `_chat_parse`/`_antwort_parse` und endete dort als „keine Vorschläge, keine Antwort" — exakt
    wie eine Nachricht, zu der das Modell nichts zu sagen hatte. Nichts wurde rot, im Audit stand
    `antwortlaenge=0`, und niemand konnte die beiden Fälle auseinanderhalten."""
    gesendet = _aufzeichnend(monkeypatch, [_hülle("")])
    with pytest.raises(LC.LlmNichtVerfuegbar) as e:
        LC._call([{"role": "user", "content": "x"}], schema=api_llm.DIALOG_SCHEMA)

    assert getattr(e.value, "grund", "") == LC.GRUND_LEER, (
        f"Die Ausnahme trägt keinen unterscheidbaren Grund: {getattr(e.value, 'grund', None)!r} — "
        f"dann kann der Aufrufer den Fall nicht benennen.")
    assert len(gesendet) == LC._VERSUCHE, (
        f"{len(gesendet)} Versuche statt {LC._VERSUCHE} — die Wiederholung greift hier nicht.")


def test_leere_antwort_ist_im_audit_unterscheidbar(konfiguriert, monkeypatch, tmp_path):
    """Ohne Eintrag stünde im Protokoll GAR NICHTS: ein Aufruf, der dreimal leer zurückkam, wäre
    von einem, der nie stattfand, nicht zu unterscheiden."""
    monkeypatch.setattr(AUDIT, "AUDIT_DIR", str(tmp_path))
    _aufzeichnend(monkeypatch, [_hülle("", provider="StreamLake")])
    with pytest.raises(LC.LlmNichtVerfuegbar):
        api_llm._llm_dialog("Ich habe 62000 Euro verdient.", KATALOG, user_id="prüfer")

    zeilen = _audit_zeilen(tmp_path)
    assert zeilen, ("kein Audit-Eintrag für den fehlgeschlagenen Aufruf — der Fall ist unsichtbar")
    assert LC.GRUND_LEER in zeilen[-1], (
        f"Der Grund steht nicht im Eintrag: {zeilen[-1]!r}")
    assert "StreamLake" in zeilen[-1], (
        f"Auch im Fehlerfall gehört der Anbieter dazu — sonst ist nicht zu sehen, WO es leer "
        f"blieb: {zeilen[-1]!r}")


def test_abgeschnittene_antwort_bekommt_genau_einen_wiederholungsversuch(konfiguriert, monkeypatch):
    """Bis 2026-08-28 stand hier das Gegenteil ("wird nicht wiederholt"), mit einer Begründung,
    die zwingend klang und an den eigenen Daten widerlegt ist: bei temperature=0 laufe derselbe
    Aufruf in dieselbe Grenze, ein zweiter Versuch koste nur Geld für dieselbe halbe Zeichenkette.
    Gemessen (s. llm_client._Abgeschnitten): derselbe Nutzertext lief 17 Minuten VOR diesem Ausfall
    erfolgreich durch, und dieselbe Eingabe erzeugt bei Stufe 2 mal 5, mal 60 Regel-Zuordnungen
    (Faktor 11) — temperature=0 heißt „nimm das wahrscheinlichste Token", nicht deterministisch.
    Ein zweiter Versuch heilt das oft.

    Trotzdem nur EINER, nicht die drei der übrigen vorübergehenden Fehler: ein abgeschnittener
    Aufruf hat bis zur Token-Grenze erzeugt und ist damit per Konstruktion der langsamste, den es
    gibt (gemessen 187,8 s) — drei davon wären für die Nutzerin ein leerer Bildschirm über zehn
    Minuten."""
    halb = '{"vorschlaege": [{"feld_id": "bruttoarb'
    gesendet = _aufzeichnend(monkeypatch, [_hülle(halb, finish="length")])
    with pytest.raises(LC.LlmNichtVerfuegbar) as e:
        LC._call([{"role": "user", "content": "x"}], schema=api_llm.DIALOG_SCHEMA)

    assert getattr(e.value, "grund", "") == LC.GRUND_ABGESCHNITTEN, (
        f"abgeschnitten und leer sind derselbe Fall geworden: {getattr(e.value, 'grund', None)!r}")
    assert len(gesendet) == 2, (
        f"{len(gesendet)} Versuche für eine abgeschnittene Antwort — erwartet ist genau EIN "
        f"Wiederholungsversuch (gemessen: derselbe Text lief 17 Minuten vorher erfolgreich durch), "
        f"nicht keiner und nicht drei wie bei den übrigen vorübergehenden Fehlern.")


def test_leer_und_abgeschnitten_zugleich_heisst_abgeschnitten(konfiguriert, monkeypatch):
    """Der Fall, den erst die Mutationsprobe sichtbar gemacht hat — und der ohne die gemessene
    Token-Aufteilung gar nicht als Fall erkennbar wäre. Beim echten Aufruf am 2026-08-21 gingen
    1185 der 1508 Antwort-Tokens ins NACHDENKEN. Ein denkendes Modell kann sein ganzes Budget
    dort verbrauchen: dann ist der Inhalt LEER **und** `finish_reason` ist `length`.

    Beide Prüfungen greifen, aber sie sagen Verschiedenes. Fiele die Entscheidung auf „leer",
    würde dreimal derselbe Aufruf abgesetzt (Budget zu klein → wieder zu klein), und im Protokoll
    stünde ein Anbieterfehler, wo unsere eigene Grenze zu eng war. Deshalb liegt `length` zuerst.

    Die Anzahl der Versuche ist trotzdem dieselbe wie bei einem sauber abgeschnittenen Fall (s.
    test_abgeschnittene_antwort_bekommt_genau_einen_wiederholungsversuch): genau EIN
    Wiederholungsversuch, gemessen und nicht bloß behauptet — dieselbe Eingabe lief 17 Minuten
    vor diesem Ausfall erfolgreich durch, ein zweiter Versuch heilt das oft."""
    gesendet = _aufzeichnend(monkeypatch, [_hülle("", finish="length")])
    with pytest.raises(LC.LlmNichtVerfuegbar) as e:
        LC._call([{"role": "user", "content": "x"}], schema=api_llm.DIALOG_SCHEMA)

    assert getattr(e.value, "grund", "") == LC.GRUND_ABGESCHNITTEN, (
        f"leer UND abgeschnitten wird als {getattr(e.value, 'grund', None)!r} gemeldet — die "
        f"engere Diagnose (unser Token-Budget) geht verloren, es sieht nach Anbieterfehler aus.")
    assert len(gesendet) == 2, (
        f"{len(gesendet)} Versuche — erwartet ist genau EIN Wiederholungsversuch, wie beim "
        f"sauber abgeschnittenen Fall, nicht keiner und nicht drei.")


def test_anbieter_eines_frueheren_aufrufs_wird_nicht_weitergemeldet(konfiguriert, monkeypatch):
    """Ein Protokoll, das einen VERALTETEN Anbieter meldet, ist schlimmer als eins, das keinen
    meldet: es sieht aus wie eine Messung. Scheitert ein Aufruf schon am HTTP, hat NIEMAND
    geantwortet — dann darf dort nicht der Name aus dem Aufruf davor stehen, ausgerechnet in der
    Zeile, die klären soll, wer geantwortet hat."""
    import io
    import urllib.error

    zustand = {"scheitern": False}

    def _urlopen(req, timeout=None):
        if zustand["scheitern"]:
            raise urllib.error.HTTPError(req.full_url, 500, "kaputt", {}, io.BytesIO(b"weg"))
        return _Antwort(_hülle(_dialog_json(antwort="da"), provider="StreamLake"))

    monkeypatch.setattr(LC.urllib.request, "urlopen", _urlopen)
    LC._call([{"role": "user", "content": "x"}], schema=api_llm.DIALOG_SCHEMA)
    assert LC.letzte_meta().get("provider") == "StreamLake", "Vorbedingung verfehlt"

    zustand["scheitern"] = True
    with pytest.raises(LC.LlmNichtVerfuegbar):
        LC._call([{"role": "user", "content": "x"}], schema=api_llm.DIALOG_SCHEMA)
    assert LC.letzte_meta().get("provider", "") == "", (
        f"Nach einem Aufruf ohne jede Antwort steht noch "
        f"{LC.letzte_meta().get('provider')!r} in den Metadaten — der Anbieter des VORIGEN "
        f"Aufrufs würde als der des gescheiterten protokolliert.")


def test_halbes_json_ohne_length_bleibt_wie_bisher(konfiguriert, monkeypatch):
    """DOKUMENTIERTE GRENZE, kein Versehen. Nur ein ausdrückliches `finish_reason: "length"`
    macht die Abschneidung erkennbar. Kommt halbes JSON mit `stop` zurück, greift weiter die
    alte Regel: `_chat_parse` liefert [] und `_antwort_parse` ("", False) — kein Müll-Vorschlag,
    aber auch kein eigener Name für den Fall. Den Inhalt selbst zu bewerten wäre Aufgabe des
    Aufrufers, der das Schema kennt; der Client kennt nur den Call."""
    _aufzeichnend(monkeypatch, [_hülle('{"vorschlaege": [{"feld_id": "bruttoarb', finish="stop")])
    text = LC._call([{"role": "user", "content": "x"}], schema=api_llm.DIALOG_SCHEMA)
    assert api_llm._chat_parse(text) == []
    assert api_llm._antwort_parse(text) == ("", False)
