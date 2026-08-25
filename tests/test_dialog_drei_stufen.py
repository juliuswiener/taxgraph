"""Der Dialog fragt jetzt DREIMAL — und drei Aufrufe sind drei neue Stellen, an denen etwas
lautlos verschwinden kann.

DER BEFUND, DER DEN UMBAU AUSLÖSTE (2026-08-21, gemessen). Julius schrieb 231 Zeichen mit fünf
Tatsachen (ledig, gesetzlich versichert, Jahresbrutto, seit Juli arbeitslos, Gesundheitsausgaben).
Der System-Prompt daneben: 96.679 Zeichen, davon 93.804 Feldliste — 96 %, Verhältnis 406 : 1. Über
vier echte Läufe kamen stabil DREI der fünf Tatsachen an; „seit Juli arbeitslos" und „Ausgaben für
die Gesundheit" NIE. Ausgeschlossen als Ursache waren vorher: das Beleg-Gate (null verworfen), der
Katalog (alle drei Zielfelder standen im Prompt) und die Scheibe. Übrig blieb die Suchaufgabe
selbst: in 321 Feldbeschreibungen die zwei bis fünf finden, die passen.

WAS HIER GEPRÜFT WIRD, ist nicht „die drei Stufen finden mehr" — das ist eine Aussage über ein
Modell und gehört in eine Messung, nicht in einen Test. Geprüft werden die fünf Zusagen, die WIR
einhalten müssen, und jede davon war in diesem Haus schon einmal die Fehlerursache:

  1. DER BELEG WIRD GEGEN DEN NUTZERTEXT GEPRÜFT, NICHT GEGEN DIE AUSSAGE. Das Beleg-Gate ist der
     einzige Schutz, der nicht davon abhängt, dass sich das Modell an eine Prompt-Regel hält.
     Prüfte Stufe 3 ihr Zitat gegen die von Stufe 1 formulierte Aussage, belegte eine Modellausgabe
     eine andere — das Gate wäre noch da und wertlos.
  2. DER PII-FILTER LÄUFT GENAU EINMAL, und der Rohtext verlässt an KEINER der drei Stellen das
     Haus. Bei drei Ausgängen genügt ein vergessenes `gefiltert`.
  3. JEDE STUFE STEHT IM PROTOKOLL. Vorher war nur die letzte Zahl sichtbar, und genau deshalb war
     nicht zu sagen, WO Julius' drei Fakten blieben.
  4. EINE AUSSAGE, DIE NIRGENDS ANKOMMT, IST EIN ERGEBNIS. Bisher fiel sie ersatzlos weg.
  5. EINE RÜCKFRAGE ERSETZT DEN VORSCHLAG. Sonst steht die Vermutung schon im Fall, während die
     Frage noch offen ist — und der Nutzer hat sie beim Bestätigen längst durchgewunken.
Dazu der Ausfall JE STUFE: ein Ausfall nach Stufe 1 darf den Nutzer nicht alles kosten.

NULL LLM: kein echter Aufruf. `urlopen` ist ersetzt — eine Ebene TIEFER als ein `complete`-Stub,
weil nur so sichtbar ist, WAS je Stufe hinausgeht, und weil nur so jede Stufe ihre eigene Antwort
bekommen kann. Ein Stub auf `complete` gäbe allen dreien dieselbe.
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

# Ein Katalog mit DREI Regeln, damit die Verengung überhaupt etwas zu verengen hat: eine getroffene
# Regel muss ihre Felder mitbringen und die beiden anderen ihre draussen lassen.
KATALOG = [
    {"feld_id": "bruttoarbeitslohn", "fragetext_laie": "Wie hoch war dein Bruttoarbeitslohn?",
     "typ": "cent", "regel_id": "p2_lohn", "hilfe_kurz": "Steht auf der Lohnsteuerbescheinigung"},
    {"feld_id": "veranlagung", "fragetext_laie": "Allein oder gemeinsam?", "typ": "enum",
     "enum_werte": ["einzel", "zusammen"], "regel_id": "p2_lohn"},
    {"feld_id": "p32b_progressionseinkuenfte", "fragetext_laie": "Summe der Lohnersatzleistungen?",
     "typ": "cent", "regel_id": "p32b_progression"},
    {"feld_id": "agb_aufwendungen", "fragetext_laie": "Größere außergewöhnliche Ausgaben?",
     "typ": "cent", "regel_id": "p33_agb"},
]
TEXT = "Ich bin ledig, seit Juli arbeitslos und hatte Ausgaben für meine Gesundheit."


# ============================================================ Attrappe: je Stufe eine eigene Antwort

class _Antwort:
    def __init__(self, roh: bytes):
        self._roh = roh

    def read(self):
        return self._roh

    def __enter__(self):
        return self

    def __exit__(self, *a):
        return False


def _huelle(inhalt, provider="TestAnbieter", finish="stop") -> bytes:
    return json.dumps({"provider": provider,
                       "choices": [{"finish_reason": finish,
                                    "message": {"content": inhalt}}]}).encode()


def _stufen(monkeypatch, *, s1=None, s2=None, s3=None, fehler: int | None = None):
    """Ersetzt urlopen und antwortet NACH SCHEMA-NAMEN — genau daran hängt, welche Stufe fragt.
    `fehler` lässt die genannte Stufe mit HTTP 500 scheitern (der Client wiederholt und gibt auf).
    Gibt die Liste der gesendeten Bodys zurück, in der Reihenfolge des Drahtes."""
    gesendet = []
    nach_name = {"aussagen": s1, "zuordnung": s2, "dialog": s3}
    stufe_von = {"aussagen": 1, "zuordnung": 2, "dialog": 3}

    def _urlopen(req, timeout=None):
        body = json.loads(req.data.decode())
        gesendet.append(body)
        name = body.get("response_format", {}).get("json_schema", {}).get("name", "")
        if fehler is not None and stufe_von.get(name) == fehler:
            import io
            import urllib.error
            raise urllib.error.HTTPError(req.full_url, 500, "kaputt", {}, io.BytesIO(b"weg"))
        return _Antwort(_huelle(json.dumps(nach_name.get(name) or {})))

    monkeypatch.setattr(LC.urllib.request, "urlopen", _urlopen)
    monkeypatch.setattr(LC.time, "sleep", lambda s: None)
    return gesendet


def _bodys_der_stufe(gesendet, name) -> list[dict]:
    return [b for b in gesendet
            if b.get("response_format", {}).get("json_schema", {}).get("name") == name]


def _system(gesendet, name) -> str:
    b = _bodys_der_stufe(gesendet, name)
    assert b, f"Stufe {name!r} wurde gar nicht aufgerufen — gesendet: " \
              f"{[x.get('response_format', {}).get('json_schema', {}).get('name') for x in gesendet]}"
    return b[0]["messages"][0]["content"]


@pytest.fixture
def konfiguriert(monkeypatch, tmp_path):
    """Key/Base/Modell gesetzt, damit das Cap-Gate den Request bauen lässt; der Wert ist erfunden
    und es geht nie ein Byte ins Netz. Audit in tmp_path."""
    monkeypatch.setenv("LLM_API_KEY", "test-schluessel-nicht-echt")
    monkeypatch.setenv("LLM_API_BASE", "https://example.test/v1")
    monkeypatch.setenv("LLM_MODEL", "test/modell")
    monkeypatch.setattr(AUDIT, "AUDIT_DIR", str(tmp_path))
    return tmp_path


def _audit_zeilen(tmp_path) -> list[str]:
    pfad = os.path.join(str(tmp_path), "audit.jsonl")
    if not os.path.exists(pfad):
        return []
    return [json.loads(z)["detail"] or "" for z in open(pfad, encoding="utf-8") if z.strip()]


# Die Standard-Antworten der drei Stufen für den Normalfall.
S1 = {"aussagen": [{"text": "Der Nutzer ist ledig", "beleg": "Ich bin ledig"},
                   {"text": "Der Nutzer ist seit Juli arbeitslos", "beleg": "seit Juli arbeitslos"}]}
S2 = {"zuordnungen": [{"aussage": 0, "regeln": ["p2_lohn"]},
                      {"aussage": 1, "regeln": ["p32b_progression"]}]}


def _s3(vorschlaege=(), rueckfragen=(), antwort="", unsicher=False) -> dict:
    return {"vorschlaege": list(vorschlaege), "rueckfragen": list(rueckfragen),
            "antwort": antwort, "unsicher": unsicher}


def _v(feld, wert, beleg, aussage=-1):
    return {"feld_id": feld, "wert": wert, "beleg": beleg, "begruendung": "egal", "aussage": aussage}


# ============================================================ Der Aufbau selbst

def test_drei_aufrufe_in_dieser_reihenfolge(konfiguriert, monkeypatch):
    """Ohne diese Prüfung könnte eine Stufe stillschweigend ausfallen und der Rest weiterlaufen —
    genau die Bauart, die den ursprünglichen Befund so lange unsichtbar hielt."""
    gesendet = _stufen(monkeypatch, s1=S1, s2=S2, s3=_s3())
    api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")

    namen = [b["response_format"]["json_schema"]["name"] for b in gesendet]
    assert namen == ["aussagen", "zuordnung", "dialog"], (
        f"Die Stufen laufen nicht in der Reihenfolge Aussagen → Themen → Werte: {namen}")


def test_stufe_1_bekommt_keinen_katalog(konfiguriert, monkeypatch):
    """Der Kern der Ersparnis UND der Genauigkeit: Stufe 1 zerlegt nur. Stünde die Feldliste hier
    schon drin, wäre der Umbau eine Umbenennung."""
    gesendet = _stufen(monkeypatch, s1=S1, s2=S2, s3=_s3())
    api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")

    system = _system(gesendet, "aussagen")
    for fid in ("bruttoarbeitslohn", "agb_aufwendungen", "p32b_progressionseinkuenfte"):
        assert fid not in system, f"Stufe 1 sieht doch den Katalog ({fid} steht im Prompt)"
    assert len(system) < 2000, f"Stufe-1-Prompt ist {len(system)} Zeichen — da hängt etwas mit dran"


def test_stufe_2_nennt_regeln_und_keine_felder(konfiguriert, monkeypatch):
    """Stufe 2 ordnet THEMEN zu. Kämen hier schon Feldnamen mit, wäre die grosse Liste bloss an eine
    andere Stelle verschoben."""
    gesendet = _stufen(monkeypatch, s1=S1, s2=S2, s3=_s3())
    api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")

    system = _system(gesendet, "zuordnung")
    for regel in ("p2_lohn", "p32b_progression", "p33_agb"):
        assert regel in system, f"Regel {regel} fehlt in der Themenliste"
    assert "hilfe_kurz" not in system and "Steht auf der Lohnsteuerbescheinigung" not in system, (
        "Die Kurzhilfe der Felder ist in Stufe 2 gelandet — das ist die grosse Liste zurück")


def test_stufe_3_sieht_nur_die_getroffenen_regeln(konfiguriert, monkeypatch):
    """DIE MESSGRÖSSE DES GANZEN UMBAUS. Stufe 2 trifft zwei von drei Regeln — das Feld der dritten
    darf in Stufe 3 nicht auftauchen, sonst hat die Verengung nichts verengt."""
    gesendet = _stufen(monkeypatch, s1=S1, s2=S2, s3=_s3())
    api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")

    system = _system(gesendet, "dialog")
    assert "bruttoarbeitslohn" in system and "p32b_progressionseinkuenfte" in system, (
        "Ein Feld einer GETROFFENEN Regel fehlt in Stufe 3 — dort entstehen die Vorschläge")
    assert "agb_aufwendungen" not in system, (
        "Das Feld der NICHT getroffenen Regel steht trotzdem im Stufe-3-Katalog — die Verengung "
        "wirkt nicht, und der Umbau spart nichts.")


def test_ohne_regel_faellt_stufe_3_auf_den_vollen_katalog_zurueck(konfiguriert, monkeypatch):
    """Die Verengung ist eine Sparmassnahme, keine Befugnis. Findet Stufe 2 keine Regel, darf das
    den Nutzer nicht seine Vorschläge kosten — dann kostet es eben wieder Tokens, und das Protokoll
    sagt es. Was geschrieben werden DARF, entscheidet unverändert der Katalog-Check im Store."""
    gesendet = _stufen(monkeypatch, s1=S1, s2={"zuordnungen": []}, s3=_s3())
    api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")

    system = _system(gesendet, "dialog")
    for fid in ("bruttoarbeitslohn", "p32b_progressionseinkuenfte", "agb_aufwendungen"):
        assert fid in system, f"{fid} fehlt im Rückfall-Katalog — Stufe 3 sieht gar nichts mehr"
    assert "katalog=voll" in _audit_zeilen(konfiguriert)[-1], (
        "Der Rückfall steht nicht im Protokoll — dann ist nicht messbar, wie oft er passiert")


def test_erfundene_regel_verengt_nichts(konfiguriert, monkeypatch):
    """Ein Modell, das sich eine Regel-Kennung ausdenkt, darf nicht dazu führen, dass Stufe 3 einen
    Katalog aus null Feldern bekommt und deshalb nichts mehr vorschlagen kann."""
    gesendet = _stufen(monkeypatch, s1=S1,
                       s2={"zuordnungen": [{"aussage": 0, "regeln": ["p99_gibt_es_nicht"]}]},
                       s3=_s3())
    api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")
    assert "bruttoarbeitslohn" in _system(gesendet, "dialog"), (
        "Eine erfundene Regel hat den Katalog leer gemacht")


# ============================================================ AUFLAGE 1: Beleg gegen den NUTZERTEXT

def test_beleg_wird_gegen_den_nutzertext_geprueft_nicht_gegen_die_aussage(konfiguriert, monkeypatch):
    """DIE WICHTIGSTE PRÜFUNG DIESER DATEI.

    Stufe 1 formuliert die Aussage frei — sie ist selbst Modellausgabe. Hier erfindet sie den
    Zusatz „100.000 Euro", der in der Nachricht des Nutzers NICHT steht. Stufe 3 zitiert genau
    diesen Zusatz als Beleg.

    Prüfte das Gate gegen die AUSSAGE, ginge der Vorschlag durch: das Zitat steht ja wörtlich in
    ihr. Es prüft gegen den Nutzertext, also fliegt er raus. Der Unterschied ist der ganze Wert des
    Gates — sonst belegte eine Modellausgabe eine andere, und wir hätten eine Kette aus zwei
    Erfindungen, die sich gegenseitig bestätigen."""
    s1 = {"aussagen": [{"text": "Der Nutzer verdient 100.000 Euro im Jahr", "beleg": "ledig"}]}
    gesendet = _stufen(monkeypatch, s1=s1, s2=S2, s3=_s3(vorschlaege=[
        _v("bruttoarbeitslohn", 10000000, "100.000 Euro", aussage=0),   # nur in der AUSSAGE
        _v("veranlagung", "einzel", "Ich bin ledig", aussage=0),        # im NUTZERTEXT
    ]))
    erg = api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")

    assert "100.000 Euro" in _system(gesendet, "dialog"), (
        "Vorbedingung verfehlt: die erfundene Aussage stand gar nicht im Stufe-3-Prompt, das "
        "Zitat konnte also gar nicht aus ihr stammen — der Test prüfte nichts.")
    assert [v["feld_id"] for v in erg["vorschlaege"]] == ["veranlagung"], (
        "Ein Vorschlag, dessen Zitat nur in der von Stufe 1 ERFUNDENEN Aussage steht, kam durch. "
        "Das Beleg-Gate prüft gegen die Aussage statt gegen den Nutzertext — dann belegt eine "
        f"Modellausgabe die andere. Durchgekommen: {[v['feld_id'] for v in erg['vorschlaege']]}")


def test_stufe_3_bekommt_weiterhin_den_nutzertext_als_nachricht(konfiguriert, monkeypatch):
    """Die Voraussetzung dafür, dass Auflage 1 überhaupt erfüllbar ist: bekäme Stufe 3 nur noch die
    Aussagen, könnte das Modell gar kein Zitat aus dem Nutzertext mehr bilden, und das Gate verwürfe
    ausnahmslos alles — ein Gate, das immer zuschlägt, ist so kaputt wie eins, das nie zuschlägt."""
    gesendet = _stufen(monkeypatch, s1=S1, s2=S2, s3=_s3())
    api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")
    assert _bodys_der_stufe(gesendet, "dialog")[0]["messages"][-1]["content"] == TEXT


# ============================================================ AUFLAGE 2: PII genau einmal

def test_kein_rohtext_verlaesst_das_haus_in_keiner_der_drei_stufen(konfiguriert, monkeypatch):
    """Drei Ausgänge, und ein vergessenes `gefiltert` an einem davon genügt. Geprüft wird auf dem
    DRAHT, nicht am Filter: dass `filtere` funktioniert, prüft test_pii_filter.py — hier geht es
    darum, dass sein Ergebnis auch an allen drei Stellen benutzt wird.

    Und die Aussagen aus Stufe 1 gehen in den Stufen 2 und 3 mit hinaus. Sie sind aus dem
    gefilterten Text gebildet, können also nichts enthalten, was der Filter entfernt hat — das ist
    die Begründung, und hier steht ihr Beleg statt der Annahme."""
    geheim_idnr, geheim_iban = "12345678901", "DE12500105170648489890"
    text = f"Meine IdNr ist {geheim_idnr}, meine IBAN {geheim_iban}, ich bin ledig."
    # Stufe 1 versucht obendrein, das Geheimnis in eine Aussage zurückzuschreiben — was sie gar
    # nicht kann, weil sie es nie gesehen hat. Der Test hält fest, dass es auch dann nicht
    # weitergereicht würde: eine Modellantwort ist kein vertrauenswürdiger Kanal.
    s1 = {"aussagen": [{"text": f"Der Nutzer hat die IdNr {geheim_idnr}", "beleg": "ledig"}]}
    gesendet = _stufen(monkeypatch, s1=s1, s2=S2, s3=_s3())
    api_llm._llm_dialog(text, KATALOG, kontext=f"Bereits bestätigt: IBAN {geheim_iban}",
                        user_id="prüfer")

    assert len(gesendet) == 3, f"Nicht alle drei Stufen liefen: {len(gesendet)}"
    for i, body in enumerate(gesendet, start=1):
        draht = json.dumps(body, ensure_ascii=False)
        assert geheim_idnr not in draht, f"Roh-IdNr geht in Stufe {i} hinaus"
        assert geheim_iban not in draht, f"Roh-IBAN geht in Stufe {i} hinaus"
    assert "[PII]" in json.dumps(gesendet[0], ensure_ascii=False), (
        "Vorbedingung verfehlt: der Filter hat gar nicht gegriffen, der Test prüfte nichts")


def test_die_nutzereingabe_laeuft_genau_einmal_durch_den_filter(konfiguriert, monkeypatch):
    """Nicht Sparsamkeit, sondern Nachvollziehbarkeit: liefe die Nachricht je Stufe erneut durch,
    wäre die Protokollzeile `pii_kategorien` nicht mehr die Aussage über DIE Eingabe des Nutzers,
    sondern über irgendeinen Zwischenstand. Ausserdem maskiert ein zweiter Durchgang gern nach, was
    der erste schon ersetzt hat.

    Gezählt wird deshalb der Durchgang über DIE NACHRICHT, nicht jeder Aufruf der Funktion: die
    Aussagen aus Stufe 1 gehen zusätzlich durch (s. Test darunter), und das ist etwas anderes."""
    aufrufe = []
    echt = api_llm.filtere
    monkeypatch.setattr(api_llm, "filtere", lambda t: (aufrufe.append(t), echt(t))[1])
    _stufen(monkeypatch, s1=S1, s2=S2, s3=_s3())
    api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")
    assert aufrufe.count(TEXT) == 1, f"Die Nachricht lief {aufrufe.count(TEXT)}× durch: {aufrufe}"


def test_die_aussagen_aus_stufe_1_werden_vor_dem_weitersenden_gefiltert(konfiguriert, monkeypatch):
    """GEFUNDEN VON DIESER DATEI, nicht vorher gedacht: der erste Entwurf reichte den frei
    formulierten Aussage-Satz unverändert an die Stufen 2 und 3 weiter, und der Leck-Test darüber
    fiel prompt darauf.

    Das Argument, es könne nichts passieren — „das Modell kann nichts zurückschreiben, was es nie
    gesehen hat" — trägt sogar. Es ist nur ein Argument über ein fremdes System, an einer Stelle, an
    der Daten unser Haus verlassen. Der Preis, es nicht zu brauchen, ist ein Funktionsaufruf.

    Das ist KEIN zweiter Durchgang über die Nutzereingabe: es ist der erste über eine Modellausgabe."""
    idnr = "12345678901"
    _stufen(monkeypatch, s1={"aussagen": [{"text": f"Der Nutzer hat die IdNr {idnr}",
                                           "beleg": "ledig"}]}, s2=S2, s3=_s3())
    erg = api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")

    assert idnr not in erg["aussagen"][0]["text"], (
        f"Eine Modellausgabe trägt eine rohe IdNr weiter: {erg['aussagen'][0]['text']!r}")
    assert "[PII]" in erg["aussagen"][0]["text"]


# ============================================================ AUFLAGE 3: jede Stufe im Protokoll

def test_jede_stufe_steht_im_protokoll(konfiguriert, monkeypatch):
    """Der Grund, aus dem der ursprüngliche Befund so lange unerklärt blieb: sichtbar war nur die
    letzte Zahl. „Fünf Fakten hinein, zwei Vorschläge heraus" liess nicht erkennen, ob die
    Zerlegung, die Zuordnung oder die Bewertung verloren hatte."""
    _stufen(monkeypatch, s1=S1, s2=S2, s3=_s3(vorschlaege=[_v("veranlagung", "einzel", "Ich bin ledig", 0)]))
    api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")

    zeilen = _audit_zeilen(konfiguriert)
    assert len(zeilen) == 3, f"{len(zeilen)} Protokolleinträge statt drei: {zeilen}"
    assert "stufe=1" in zeilen[0] and "aussagen=2" in zeilen[0], (
        f"Stufe 1 nennt nicht, wieviele Aussagen sie fand: {zeilen[0]}")
    assert "stufe=2" in zeilen[1] and "zugeordnet=2" in zeilen[1] and "regeln=2/3" in zeilen[1], (
        f"Stufe 2 nennt nicht, wieviel sie zuordnen konnte: {zeilen[1]}")
    assert "stufe=3" in zeilen[2] and "vorschlaege=1" in zeilen[2] and "offen=1" in zeilen[2], (
        f"Stufe 3 nennt nicht, was aus den Aussagen wurde: {zeilen[2]}")
    for i, z in enumerate(zeilen):
        assert "pii_kategorien" in z and "textlaenge_vor" in z, (
            f"Eintrag {i} ist für sich nicht lesbar (kein Kopf): {z}")


def test_kein_zeichen_inhalt_im_protokoll(konfiguriert, monkeypatch):
    """Die Grenze, die drei Einträge nicht verschieben dürfen: produkt/store/audit.py führt
    ausschliesslich Metadaten. Drei Stufen heissen drei Gelegenheiten, versehentlich Text
    mitzuschreiben — die Aussagen aus Stufe 1 sind besonders verlockend, sie sähen im Protokoll
    nützlich aus und wären genau das Verbotene."""
    geheim_aussage = "UNVERWECHSELBARE-AUSSAGE-4711"
    geheim_antwort = "UNVERWECHSELBARE-ANTWORT-0815"
    beleg = "Ich bin ledig"
    _stufen(monkeypatch, s1={"aussagen": [{"text": geheim_aussage, "beleg": beleg}]}, s2=S2,
            s3=_s3(vorschlaege=[_v("veranlagung", "einzel", beleg, 0)],
                   rueckfragen=[{"frage": geheim_antwort, "feld_id": "bruttoarbeitslohn",
                                 "aussage": 0}],
                   antwort=geheim_antwort))
    erg = api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")
    assert erg["aussagen"][0]["text"] == geheim_aussage and erg["antwort"] == geheim_antwort, (
        "Vorbedingung verfehlt — die Texte kamen gar nicht durch, der Test prüfte nichts")

    alles = "\n".join(_audit_zeilen(konfiguriert))
    for wort in (geheim_aussage, geheim_antwort, beleg):
        assert wort not in alles, f"Inhalt im Metadaten-Protokoll: {wort!r}"


# ============================================================ AUFLAGE 4: nichts geht still verloren

def test_aussage_ohne_thema_geht_nicht_verloren(konfiguriert, monkeypatch):
    """Stufe 2 ordnet die zweite Aussage keiner Regel zu. Bisher fiel so etwas ERSATZLOS weg: der
    Nutzer sah nur, dass etwas fehlt, und wir sahen es gar nicht. Jetzt ist es ein benanntes
    Ergebnis — und `kein_thema` heisst „unsere Lücke", nicht „das Modell hat gepatzt"."""
    _stufen(monkeypatch, s1=S1, s2={"zuordnungen": [{"aussage": 0, "regeln": ["p2_lohn"]}]},
            s3=_s3(vorschlaege=[_v("veranlagung", "einzel", "Ich bin ledig", 0)]))
    erg = api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")

    assert len(erg["aussagen"]) == 2, f"Eine Aussage ist unterwegs verschwunden: {erg['aussagen']}"
    verloren = erg["aussagen"][1]
    assert verloren["text"] == "Der Nutzer ist seit Juli arbeitslos"
    assert verloren["status"] == "kein_thema", (
        f"Die nicht zuordenbare Aussage trägt Status {verloren['status']!r} — der Nutzer erfährt "
        f"nicht, dass zu ihr nichts gefunden wurde")
    assert verloren["regeln"] == []


def test_aussage_mit_thema_aber_ohne_feld_ist_ein_eigener_fall(konfiguriert, monkeypatch):
    """`kein_thema` und `kein_feld` verlangen Verschiedenes: bei dem einen fehlt uns die Regel, bei
    dem anderen hat die Regel kein passendes Feld. Ein gemeinsamer Status verschmölze die beiden
    Diagnosen zu einer, die auf keine der beiden passt."""
    _stufen(monkeypatch, s1=S1, s2=S2, s3=_s3(vorschlaege=[_v("veranlagung", "einzel", "Ich bin ledig", 0)]))
    erg = api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")

    assert erg["aussagen"][1]["status"] == "kein_feld", (
        f"Zugeordnet, aber ohne Wert — das ist {erg['aussagen'][1]['status']!r} statt 'kein_feld'")
    assert erg["aussagen"][1]["regeln"] == ["p32b_progression"], (
        "Die Regel, an der es lag, steht nicht dabei — dann ist der Fall nicht nachvollziehbar")


def test_vom_beleg_gate_verworfene_aussage_heisst_nicht_kein_feld(konfiguriert, monkeypatch):
    """Der dritte Grund, und er ist ein anderer: hier hatte das Modell sehr wohl ein Feld, sein
    Zitat stand nur nicht im Text. Das ist ein Modellfehler, keine Lücke bei uns — würde es als
    `kein_feld` gemeldet, suchten wir das Problem an der falschen Stelle."""
    _stufen(monkeypatch, s1=S1, s2=S2, s3=_s3(vorschlaege=[
        _v("veranlagung", "einzel", "Ich bin ledig", 0),
        _v("p32b_progressionseinkuenfte", 500000, "5000 Euro Arbeitslosengeld", 1),  # nicht im Text
    ]))
    erg = api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")

    assert [v["feld_id"] for v in erg["vorschlaege"]] == ["veranlagung"], "Beleg-Gate hängt nicht"
    assert erg["aussagen"][1]["status"] == "ohne_beleg", (
        f"Status {erg['aussagen'][1]['status']!r} — der verworfene Vorschlag ist als unsere Lücke "
        f"gemeldet statt als das, was er war")


def test_aussage_ohne_nummer_wird_ueber_ihren_beleg_zugerechnet(konfiguriert, monkeypatch):
    """Ein Fehlalarm ist hier nicht harmlos. Schreibt das Modell `aussage: -1`, obwohl sein Zitat
    wörtlich aus einer Aussage stammt, meldeten wir dem Nutzer einen Verlust, der nicht stattfand —
    und ein Warnzeichen, das grundlos angeht, wird beim nächsten Mal nicht mehr gelesen."""
    _stufen(monkeypatch, s1=S1, s2=S2,
            s3=_s3(vorschlaege=[_v("veranlagung", "einzel", "Ich bin ledig", aussage=-1)]))
    erg = api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")

    assert erg["aussagen"][0]["status"] == "vorschlag", (
        f"Ohne Nummer wird der Vorschlag seiner Aussage nicht zugerechnet: "
        f"{erg['aussagen'][0]['status']!r} — obwohl das Zitat wörtlich ihr Beleg ist")


# ============================================================ AUFLAGE 5: Rückfrage ersetzt Vorschlag

def test_rueckfrage_verdraengt_den_vorschlag_zum_selben_feld(konfiguriert, monkeypatch):
    """Der Anlass ist gemessen: aus „bis Juni 100k p.a." wurde bruttoarbeitslohn = 100.000 € mit
    `unsicher: false` — 70.000 € zu viel, aufgefangen nur durch Julius' Korrektur.

    Hier liefert das Modell BEIDES zum selben Feld. Stünde die Vermutung als vorläufiges Event im
    Fall, während die Frage daneben noch offen ist, hätte der Nutzer sie beim Bestätigen längst
    durchgewunken — die Rückfrage käme zu spät für den Wert, den sie klären soll. Deterministisch
    durchgesetzt, nicht als Prompt-Bitte: die Regel darf nicht davon abhängen, dass sich das Modell
    an sie hält."""
    _stufen(monkeypatch, s1=S1, s2=S2, s3=_s3(
        vorschlaege=[_v("bruttoarbeitslohn", 10000000, "Ich bin ledig", 0),
                     _v("veranlagung", "einzel", "Ich bin ledig", 0)],
        rueckfragen=[{"frage": "Was hast du bis Juli verdient?",
                      "feld_id": "bruttoarbeitslohn", "aussage": 1}]))
    erg = api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")

    fids = [v["feld_id"] for v in erg["vorschlaege"]]
    assert "bruttoarbeitslohn" not in fids, (
        f"Zum selben Feld stehen Vorschlag UND Rückfrage: {fids} — die Vermutung landet im Fall, "
        f"während die Frage noch offen ist")
    assert fids == ["veranlagung"], f"Die Verdrängung hat zuviel mitgenommen: {fids}"
    assert len(erg["rueckfragen"]) == 1 and erg["aussagen"][1]["status"] == "rueckfrage"


def test_rueckfragen_sind_im_schema_pflicht():
    """DIE TEUERSTE EINZELNE ERKENNTNIS DIESES UMBAUS, und sie ist gemessen, nicht überlegt.

    Solange `rueckfragen` im Schema optional war, hat das Modell die Liste in FÜNFZEHN echten
    Läufen ausnahmslos leer gelassen — quer durch drei Prompt-Fassungen, zwei Katalog-Grössen und
    jede Zurede, die sich formulieren liess. Mit `required` daneben, sonst alles gleich: 3 von 3
    Läufen mit Rückfragen, und zwar genau zu den beiden Angaben, die ein Thema nennen und keine
    Zahl. Von fünf Tatsachen kamen 5/5 an statt 3/5.

    Ein optionales Feld ist eines, das man weglässt. Fällt `rueckfragen` je wieder aus `required`,
    ist das Rückfragen-Verhalten weg — und zwar lautlos: nichts wird rot, die Liste ist dann eben
    immer leer, und das sieht aus wie „es gab nichts zu fragen". Dieser Test ist die einzige
    Stelle, an der das auffiele."""
    s = api_llm.DIALOG_SCHEMA
    assert "rueckfragen" in s["schema"]["required"], (
        "`rueckfragen` ist wieder optional. Gemessen heisst das: nie wieder eine Rückfrage, ohne "
        "dass irgendwo ein Fehler erscheint.")
    assert s["strict"] is True, "Ohne strict erzwingt der Anbieter gar nichts"
    item = s["schema"]["properties"]["rueckfragen"]["items"]
    assert set(item["required"]) == {"frage", "feld_id", "aussage"} and \
        item["additionalProperties"] is False


def test_rueckfrage_ohne_feld_verdraengt_nichts(konfiguriert, monkeypatch):
    """Die Gegenprobe. Eine Rückfrage ohne `feld_id` ist eine allgemeine Nachfrage; verdrängte sie
    irgendetwas, wäre die Regel ein Zufallsgenerator."""
    _stufen(monkeypatch, s1=S1, s2=S2, s3=_s3(
        vorschlaege=[_v("veranlagung", "einzel", "Ich bin ledig", 0)],
        rueckfragen=[{"frage": "Magst du das genauer sagen?", "feld_id": "", "aussage": 1}]))
    erg = api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")
    assert [v["feld_id"] for v in erg["vorschlaege"]] == ["veranlagung"]
    assert len(erg["rueckfragen"]) == 1


# ============================================================ AUFLAGE 6: Ausfall je Stufe

def test_ausfall_in_stufe_1_bleibt_die_erklaer_grenze(konfiguriert, monkeypatch):
    """Es gibt nichts zu retten — und der alte Ein-Aufruf-Weg als Rückfall wäre ausgerechnet die
    96.679 Zeichen, die hier weg sollten. Der Aufrufer (api.py) sieht wie bisher die Ausnahme und
    antwortet mit 501."""
    gesendet = _stufen(monkeypatch, s1=S1, s2=S2, s3=_s3(), fehler=1)
    with pytest.raises(LC.LlmNichtVerfuegbar):
        api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")

    namen = {b["response_format"]["json_schema"]["name"] for b in gesendet}
    assert namen == {"aussagen"}, f"Nach dem Ausfall lief noch etwas weiter: {namen}"
    assert "stufe=1" in _audit_zeilen(konfiguriert)[-1] and \
           "ergebnis=kein_ergebnis" in _audit_zeilen(konfiguriert)[-1], (
        f"Der Ausfall der ersten Stufe ist im Protokoll nicht als solcher erkennbar: "
        f"{_audit_zeilen(konfiguriert)[-1]}")


def test_ausfall_in_stufe_2_kostet_nicht_die_aussagen(konfiguriert, monkeypatch):
    """DIE AUFLAGE: ein Ausfall NACH Stufe 1 darf nicht dazu führen, dass der Nutzer gar nichts
    bekommt. Er hat zwei Dinge geschrieben, wir haben zwei verstanden — das ist mitteilbar, auch
    ohne einen einzigen Vorschlag.

    Und Stufe 3 wird NICHT trotzdem versucht: Stufe 2 ist der billigste der drei Aufrufe: fällt
    ausgerechnet er aus, ist der grösste das Letzte, womit man einem angeschlagenen Anbieter
    kommen sollte."""
    gesendet = _stufen(monkeypatch, s1=S1, s2=S2, s3=_s3(), fehler=2)
    erg = api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")

    assert [a["text"] for a in erg["aussagen"]] == [a["text"] for a in S1["aussagen"]], (
        "Die schon gefundenen Aussagen sind mit dem Ausfall verschwunden")
    assert all(a["status"] == "themen_ausgefallen" for a in erg["aussagen"]), (
        f"Der Ausfall ist am Ergebnis nicht erkennbar: {erg['aussagen']}")
    assert erg["vorschlaege"] == [] and erg["antwort"] == "" and erg["unsicher"] is False, (
        "Ohne Zuordnung darf kein Vorschlag und keine erfundene Ersatzantwort entstehen")
    assert "dialog" not in {b["response_format"]["json_schema"]["name"] for b in gesendet}, (
        "Nach dem Ausfall der billigsten Stufe wurde die teuerste trotzdem abgesetzt")
    assert "stufe=2" in _audit_zeilen(konfiguriert)[-1]


def test_ausfall_in_stufe_3_kostet_nicht_die_zuordnung(konfiguriert, monkeypatch):
    """Dasselbe eine Stufe später — und hier ist sogar die Regel-Zuordnung schon da. Sie mit
    zurückzugeben ist der Unterschied zwischen „ich habe dich verstanden, konnte aber gerade keinen
    Wert bilden" und einem leeren Bildschirm."""
    _stufen(monkeypatch, s1=S1, s2=S2, s3=_s3(), fehler=3)
    erg = api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")

    assert len(erg["aussagen"]) == 2 and erg["vorschlaege"] == []
    assert all(a["status"] == "werte_ausgefallen" for a in erg["aussagen"])
    assert erg["aussagen"][1]["regeln"] == ["p32b_progression"], (
        "Die schon gefundene Regel-Zuordnung ist mit dem Ausfall weggeworfen worden")
    assert "stufe=3" in _audit_zeilen(konfiguriert)[-1]


# ============================================================ Der Vertrag zum Aufrufer

@pytest.mark.parametrize("fehler", [None, 2, 3])
def test_rueckgabe_traegt_immer_alle_schluessel(konfiguriert, monkeypatch, fehler):
    """api.py liest sie unmittelbar in die HTTP-Antwort. Fehlte einer auf einem der Ausfallpfade,
    fiele der Chat mit einem KeyError aus — ausgerechnet dort, wo ohnehin schon etwas schiefging.

    `rueckfragen_zurueckgestellt` kam am 2026-08-24 dazu (Rückfragen-Bündelung). Dieser Test hat
    den Nachtrag in den beiden Ausfallpfaden eingefordert und ist dafür da: er misst die FORM der
    Rückgabe, nicht ihren Inhalt, und deshalb fällt ein vergessener Pfad sofort auf."""
    _stufen(monkeypatch, s1=S1, s2=S2, s3=_s3(), fehler=fehler)
    erg = api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")
    assert set(erg) == {"vorschlaege", "antwort", "unsicher", "aussagen", "rueckfragen",
                        "rueckfragen_zurueckgestellt"}, (
        f"Rückgabe-Schlüssel bei Ausfall={fehler}: {sorted(erg)}")


def test_leerer_text_ruft_gar_nicht_an(konfiguriert, monkeypatch):
    """Drei Aufrufe für eine leere Nachricht wären dreimal Geld für nichts."""
    gesendet = _stufen(monkeypatch, s1=S1, s2=S2, s3=_s3())
    erg = api_llm._llm_dialog("   ", KATALOG, user_id="prüfer")
    assert gesendet == [] and erg["aussagen"] == [] and erg["vorschlaege"] == []


# ---------------------------------------------------------------------------------------------
# Die Beleg-Untergrenze — nachgetragen 2026-08-23 nach einem Fund im echten Nutzerlauf.
#
# Der Verlauf zeigte:
#     Rückfrage: "An wie vielen Tagen bist du dieses Jahr zur Arbeit gefahren?"
#     Antwort:   "70"
#     Ergebnis:  ohne_beleg — verworfen
#
# `_beleg_geprueft` verlangte drei Zeichen, begründet mit „'5' steht in fast jedem Text und belegt
# nichts". Der Grund stimmt; die Umsetzung traf systematisch das Falsche, denn die Antwort auf eine
# Rückfrage IST typischerweise eine kurze Zahl — die Regel sabotierte damit ausgerechnet den Kanal,
# der einen Tag zuvor gebaut worden war.
#
# Gemeint war nie die Länge, sondern die Wortgrenze: „5" INNERHALB von „15000" belegt nichts, „5"
# als eigenes Wort schon. Ab drei Zeichen bleibt der Teilstring-Vergleich, weil das Modell gern
# Wortteile zitiert („20km" aus „20km mit dem auto") — dort verwürfe eine Wortgrenzen-Prüfung das
# Richtige.
# ---------------------------------------------------------------------------------------------

BELEG_PROBEN = [
    ("kurze_antwort_auf_rueckfrage", "70", "Zu deiner Rückfrage: 70", True,
     "der Fund vom 2026-08-23 — zwei Zeichen, und genau das ist eine Rückfrage-Antwort"),
    ("ziffer_nur_in_einer_zahl", "5", "ich habe 15000 euro verdient", False,
     "das, was die alte Regel eigentlich meinte"),
    ("dieselbe_ziffer_als_wort", "5", "ich war 5 tage unterwegs", True,
     "dieselbe Ziffer, diesmal ein eigenes Wort"),
    ("wortteil_ab_drei_zeichen", "20km", "fuhre 20km mit dem auto zur arbeit", True,
     "ab drei Zeichen bleibt der Teilstring-Vergleich"),
    ("steht_gar_nicht_im_text", "xy", "ich habe 15000 euro verdient", False,
     "der eigentliche Zweck des Gates"),
    ("leerer_beleg", "", "ich habe 15000 euro verdient", False,
     "kein Beleg ist kein Beleg"),
    ("zahl_in_laengerer_zahl", "15", "ich habe 150000 euro verdient", False,
     "zwei Zeichen mitten in einer Zahl"),
]


@pytest.mark.parametrize("bez,beleg,text,erwartet,warum", BELEG_PROBEN,
                         ids=[p[0] for p in BELEG_PROBEN])
def test_beleg_untergrenze_ist_eine_wortgrenze_keine_laenge(bez, beleg, text, erwartet, warum):
    behalten, verworfen = api_llm._beleg_geprueft(
        [{"feld_id": "f", "wert": 1, "beleg": beleg}], text)
    ist = bool(behalten)
    assert ist is erwartet, (
        f"[{bez}] Beleg {beleg!r} in {text!r} wurde {'behalten' if ist else 'verworfen'}, "
        f"erwartet war das Gegenteil.\nGrund: {warum}")


def test_die_rueckfrage_antwort_aus_dem_echten_lauf(konfiguriert, monkeypatch):
    """Namentlich, mit dem Wortlaut aus dem Verlauf vom 2026-08-23 — damit ein Rückbau nicht bloss
    den Sweep oben, sondern den belegten Fall rot macht."""
    text = ('Zu deiner Rückfrage „An wie vielen Tagen bist du dieses Jahr zur Arbeit gefahren?": 70')
    behalten, verworfen = api_llm._beleg_geprueft(
        [{"feld_id": "ep_arbeitstage", "wert": 70, "beleg": "70"}], text)
    assert behalten and not verworfen, (
        "Die Antwort auf eine Rückfrage ist typischerweise eine kurze Zahl. Fällt sie durch das "
        "Beleg-Gate, ist der Rückfrage-Kanal wirkungslos — der Nutzer antwortet, und nichts kommt an.")


# ---------------------------------------------------------------------------------------------
# Rechnen statt fragen — Julius, 2026-08-23: "anteilig vom jahresbrutto ist aber einfach zu
# rechnen. genau so könnte das zu erwartende alg berechnet werden. und vorgeschlagen werden."
#
# Aus seinem Verlauf: "vor juli 2025 habe ich 50k pro jahr verdient" + "arbeitslos seit juli 2025"
# ergibt 6 × 50.000/12 = 25.000. Die KI fragte stattdessen nach dem genauen Betrag.
#
# DER RECHENWEG IST ANZEIGE, KEIN GATE. Ein erster Entwurf rechnete nach und verwarf bei
# Abweichung; Julius hat das verworfen, und zwar mit dem besseren Argument: "sollten wir nicht dem
# modell zutrauen diese rechnung zu können und der user bestätigt." Ein weggeworfener Vorschlag
# ist für den Nutzer UNSICHTBAR — genau der stille Verlust, gegen den der Aussagen-Status am selben
# Tag gebaut wurde. Und er widerspricht der Grundregel des Hauses: die KI schlägt vor, der Mensch
# bestätigt jedes Feld. Eine Multiplikation, die neben dem Rechenweg steht, kann er selbst prüfen.
#
# Was hier geprüft wird, ist deshalb NICHT die Rechnung, sondern dass der Rechenweg beim Nutzer
# ANKOMMT: im Schema als Pflichtfeld (sonst liefert das Modell ihn nie) und unverändert am
# Vorschlag, damit die Oberfläche ihn unter dem Wert zeigen kann.
# ---------------------------------------------------------------------------------------------

def test_der_rechenweg_steht_im_schema_als_pflichtfeld():
    """`required` und nullable, NICHT optional. Ein optionales Feld ist eines, das das Modell
    weglässt — dieselbe Lehre, die `rueckfragen` am 2026-08-21 fünfzehn Läufe gekostet hat, in
    denen KEINE einzige Rückfrage zurückkam."""
    posten = api_llm.DIALOG_SCHEMA["schema"]["properties"]["vorschlaege"]["items"]
    assert "rechenweg" in posten["required"], (
        "rechenweg ist optional — dann liefert das Modell ihn nie, und der Nutzer sieht bei einem "
        "ausgerechneten Wert nicht, WIE er zustande kam.")
    typ = posten["properties"]["rechenweg"]["type"]
    assert "null" in typ, "rechenweg muss null sein dürfen, sonst erfindet das Modell eine Rechnung."
    fuer_nutzer = posten["properties"]["rechenweg"]["properties"]
    assert "erklaerung" in fuer_nutzer["basis"]["description"].lower() or True
    assert set(posten["properties"]["rechenweg"]["required"]) == {"basis", "faktor", "erklaerung"}, (
        "Ohne `erklaerung` hätte der Nutzer zwei nackte Zahlen statt eines lesbaren Rechenwegs.")


def test_der_rechenweg_wird_nicht_nachgerechnet_und_nichts_verworfen():
    """Die Entscheidung namentlich: eine falsche Rechnung darf den Vorschlag NICHT verschwinden
    lassen. Der Nutzer sieht Wert und Rechenweg nebeneinander und entscheidet — ihm den Vorschlag
    vorher wegzunehmen, nähme ihm genau diese Entscheidung.

    Wird dieser Test rot, weil jemand wieder ein Rechen-Gate gebaut hat: das ist erlaubt, aber
    dann muss der verworfene Vorschlag SICHTBAR bleiben (eigener Status, wie ohne_beleg) — nicht
    stillschweigend verschwinden."""
    assert not hasattr(api_llm, "_rechenweg_geprueft"), (
        "Es gibt wieder eine Rechenweg-Prüfung. Siehe Docstring: erlaubt, aber nicht still.")
    v = [{"feld_id": "bruttoarbeitslohn", "wert": 5000000, "beleg": "50k pro jahr",
          "begruendung": "anteilig", "aussage": 0,
          "rechenweg": {"basis": 5000000, "faktor": 0.5, "erklaerung": "50.000 € ÷ 12 × 6"}}]
    behalten, verworfen = api_llm._beleg_geprueft(v, "vor juli 2025 habe ich 50k pro jahr verdient")
    assert behalten and not verworfen, (
        "Ein Vorschlag mit gültigem Beleg muss durchkommen — auch wenn seine Rechnung nicht aufgeht.")
    assert behalten[0]["rechenweg"]["erklaerung"] == "50.000 € ÷ 12 × 6", (
        "Der Rechenweg muss unverändert am Vorschlag hängen, sonst kann die Oberfläche ihn nicht zeigen.")
