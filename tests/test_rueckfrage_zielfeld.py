"""Die Rückfrage und das Feld, in das ihre Antwort läuft, müssen dasselbe meinen.

ANLASS, gemessen im Live-Lauf `serie-rentner-mit-behinderung-1787909863` vom 2026-08-28. Die
Nutzerin schrieb „ich bin seit 2023 in rente, bekomme 1850 euro im monat, habe einen
schwerbehindertenausweis mit GdB 60 und spende regelmaessig." Die KI fragte:

    Feld:  rentner_anzahl_renten   (typ int, „Wie viele verschiedene Renten bekommst du?")
    Text:  „Wie hoch ist der monatliche Rentenzahlbetrag genau? Du schreibst 1850 Euro – ist das
            der Netto- oder der Bruttobetrag?"

Die Oberfläche baut das Eingabefeld nach dem TYP des Feldes, nicht nach dem Fragetext. Wer die
gestellte Frage beantwortet, tippt 1850 — und legt 1850 Renten an. Kein Anzeigefehler: ein
Zahlendreher, der bestätigt und plausibel aussehend im Fall landet.

ZWEI URSACHEN, und beide werden hier geprüft.

  1. DER KATALOG. Nachgebaut aus dem Mitschnitt: Stufe 2 wählte `p22_anzahl_renten_erhebung` (die
     Pseudo-Regel des Zählfelds) und NICHT `p22_1_leibrente_besteuerungsanteil`. Der enge Katalog
     trug 22 Felder und darunter kein einziges, das einen Rentenbetrag aufnehmen kann —
     `rentner_jahresrente` war nicht dabei und wurde im ganzen Lauf nie gefragt. Die 1850 € hatten
     kein Ziel; das Modell hängte die Frage an das einzige Feld mit „Rente" im Namen.
     `_mit_zaehlfeldern` hält die Paarung jetzt in BEIDE Richtungen.

  2. DIE FEHLENDE PRÜFUNG. Auch mit dem richtigen Feld im Katalog kann das Modell danebengreifen,
     und nichts hielt es auf: `feld_id` wurde ungeprüft übernommen. `_rueckfragen_gebunden` löst
     die Bindung, wenn Frage und Feld sich widersprechen — und lässt die FRAGE stehen.

WAS DIESE DATEI AUSDRÜCKLICH MITPRÜFT: dass die Prüfung nicht mehr kaputt macht, als sie fängt.
An 50 verschiedenen Rückfragen aus echten Läufen greift sie dreimal und trifft keine richtige
Frage; die Gegenprobe dazu steht in `test_der_normalfall_wird_nicht_angetastet` mit dem Wortlaut
echter Rückfragen.

NULL LLM — `llm_client.complete` bzw. `urlopen` sind durch Fixture-Funktionen ersetzt, wie in
tests/test_rueckfragen_gebuendelt.py.
"""
from __future__ import annotations

import json
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
             "produkt/unsicherheit", "produkt/import", "golden", "produkt/auth"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api_llm                      # noqa: E402
import audit as AUDIT               # noqa: E402
import llm_client as LC             # noqa: E402
import traverser as TR              # noqa: E402

# Der Katalog, den Stufe 3 im gemessenen Lauf gesehen hat — auf das Nötige gekürzt. `typ` ist die
# Angabe, an der die Prüfung hängt: in diesem Haus ist JEDES Geldfeld `cent`, und kein `int`-Feld
# trägt eine Geld-Einheit (nachgezählt 2026-08-28: int trägt Jahr, Tage, Monate, km, %, Stunden,
# kWp, Kinder — nie EUR).
KAT = [
    {"feld_id": "rentner_anzahl_renten", "typ": "int",
     "fragetext_laie": "Wie viele verschiedene Renten bekommst du?"},
    {"feld_id": "rentner_jahresrente", "typ": "cent",
     "fragetext_laie": "Wie viel Rente hast du dieses Jahr insgesamt bekommen?"},
    {"feld_id": "rentner_grad_der_behinderung", "typ": "int",
     "fragetext_laie": "Welchen Grad der Behinderung hat dein Ausweis?"},
    {"feld_id": "spenden_betrag", "typ": "cent",
     "fragetext_laie": "Wie viel hast du gespendet?"},
    {"feld_id": "keine_spenden", "typ": "bool", "fragetext_laie": "Hast du gespendet?"},
]

# Der Wortlaut aus dem Live-Lauf, Zeichen für Zeichen.
GEMESSEN = ("Wie hoch ist der monatliche Rentenzahlbetrag genau? Du schreibst 1850 Euro – ist das "
            "der Netto- oder der Bruttobetrag?")


def _rf(frage, feld_id="", aussage=0):
    return {"frage": frage, "feld_id": feld_id, "aussage": aussage}


# ================================================================ die Prüfung selbst

def test_geldfrage_an_einem_zaehlfeld_verliert_ihr_feld():
    """DER GEMESSENE FALL. Die Frage nennt Euro, das Feld zählt Renten — wer antwortet, legt 1850
    Renten an."""
    rueck, geloest = api_llm._rueckfragen_gebunden(
        [_rf(GEMESSEN, "rentner_anzahl_renten", 1)], KAT)

    assert rueck[0]["feld_id"] == "", (
        "Die Frage nach 1850 Euro zeigt weiter auf rentner_anzahl_renten (typ int). Wer sie "
        "beantwortet, trägt 1850 als ANZAHL ein.")
    assert [g["grund"] for g in geloest] == ["zahlenart"], f"Falscher Grund: {geloest}"


def test_anzahlfrage_an_einem_geldfeld_verliert_ihr_feld():
    """Dieselbe Verwechslung andersherum, ebenfalls aus einem echten Lauf: „Wie viele Monate hast
    du die 1500 Euro pro Monat erhalten?" zeigte auf ein Feld, das die Jahressumme in Cent führt.
    Eine Antwort „6" wären sechs Cent."""
    rueck, geloest = api_llm._rueckfragen_gebunden(
        [_rf("Wie viele Monate hast du die 1500 Euro pro Monat erhalten?", "spenden_betrag", 0)],
        KAT)

    assert rueck[0]["feld_id"] == "", "Anzahl-Frage zeigt weiter auf ein Geldfeld."
    assert [g["grund"] for g in geloest] == ["zahlenart"]


def test_ein_erfundenes_feld_verliert_seine_bindung():
    """Das Schema verlangt „exakt eine feld_id aus der Liste". Gemessen 1 von 24 Rückfragen aus
    echten Läufen nannte eines, das es nicht gibt (`entfernungspauschale_tage` statt
    `ep_arbeitstage`). Heute verschwindet so eine Frage in der Oberfläche spurlos, weil das Feld
    nicht in /fragen steht — sie sieht danach aus wie eine, die der Nutzer übersprungen hat."""
    rueck, geloest = api_llm._rueckfragen_gebunden(
        [_rf("An wie vielen Tagen bist du gefahren?", "entfernungspauschale_tage", 0)], KAT)

    assert rueck[0]["feld_id"] == ""
    assert [g["grund"] for g in geloest] == ["unbekannt"], f"Falscher Grund: {geloest}"


def test_die_frage_selbst_bleibt_immer_stehen():
    """DAS WICHTIGSTE AN DER GANZEN PRÜFUNG. Die Frage nach den 1850 € ist RICHTIG — nur ihr Ziel
    ist falsch. Sie wegzuwerfen wäre der stille Verlust, gegen den dieses Modul gebaut ist: die
    Nutzerin hätte ihre einzige Einkunft genannt und würde nie danach gefragt.

    Ohne Feld zeigt die Oberfläche sie im Chat; die Antwort läuft erneut durch die drei Stufen und
    kann diesmal im richtigen Feld landen."""
    roh = [_rf(GEMESSEN, "rentner_anzahl_renten", 1),
           _rf("Frei erfunden?", "gibt_es_nicht", 2),
           _rf("Wie viele Monate?", "spenden_betrag", 3)]
    rueck, geloest = api_llm._rueckfragen_gebunden(roh, KAT)

    assert len(rueck) == 3, f"Es wurde eine Frage weggeworfen statt entkoppelt: {rueck}"
    assert [r["frage"] for r in rueck] == [r["frage"] for r in roh], "Der Wortlaut hat sich geändert"
    assert len(geloest) == 3


def test_der_normalfall_wird_nicht_angetastet():
    """DIE GEGENPROBE, ohne die „löse jede Bindung" eine bestandene Lösung wäre. Der Wortlaut
    stammt aus echten Läufen; keine dieser Fragen darf ihr Feld verlieren.

    Zwei der Zeilen stehen für je eine Grenze, die die Regel ausdrücklich NICHT überschreitet:

      * „Hast du mehr als 1000 Euro gespendet?" nennt Euro und zeigt auf ein `bool`-Feld. Kein
        Widerspruch — ein Ja/Nein-Feld nimmt keine Zahl auf, es kann also auch keine falsche
        aufnehmen. Deshalb prüft die Regel `int` gegen Geld und `cent` gegen Anzahl, nicht
        „alles, was nicht cent ist".
      * „Wie hoch ist der Grad deiner Behinderung?" fragt nach einer Zahl, die kein Geld ist.
        „Wie hoch" darf deshalb kein Geldwort sein — sonst verlöre diese völlig richtige Frage
        ihr Feld. Die Zeile steht hier, weil eine Mutationsprobe gezeigt hat, dass genau diese
        Verschlechterung sonst durch alle Tests kommt: die Grenze stand nur im Kommentar."""
    roh = [_rf("Wie viel hast du dieses Jahr insgesamt an Spenden gezahlt?", "spenden_betrag", 0),
           _rf("Welchen Grad der Behinderung hat dein Ausweis?", "rentner_grad_der_behinderung", 1),
           _rf("Hast du mehr als 1000 Euro gespendet?", "keine_spenden", 2),
           _rf("Wie viele verschiedene Renten bekommst du?", "rentner_anzahl_renten", 3),
           _rf("Wie viel Rente hast du dieses Jahr insgesamt bekommen?", "rentner_jahresrente", 4),
           _rf("Wie hoch ist der Grad deiner Behinderung?", "rentner_grad_der_behinderung", 6),
           _rf("Und wonach fragst du eigentlich?", "", 5)]
    vorher = [r["feld_id"] for r in roh]
    rueck, geloest = api_llm._rueckfragen_gebunden(roh, KAT)

    assert [r["feld_id"] for r in rueck] == vorher, (
        f"Richtige Rückfragen haben ihr Feld verloren: "
        f"{[g['frage'][:50] for g in geloest]}")
    assert geloest == [], f"Die Prüfung schlägt im Normalfall an: {geloest}"


def test_leere_liste_bleibt_leer():
    assert api_llm._rueckfragen_gebunden([], KAT) == ([], [])


# ================================================================ der Katalog: beide Richtungen

GRUPPEN = {"rente": {"gruppe": "rente", "anzahl_feld": "rentner_anzahl_renten", "max": 5}}


@pytest.fixture
def nur_rente(monkeypatch):
    """Nur eine Gruppe, damit die Tests nicht an den echten YAML-Dateien hängen. Die Paarung
    selbst (Zählfeld ↔ Gruppe) ist dieselbe."""
    monkeypatch.setattr(TR, "lade_instanz_gruppen", lambda: GRUPPEN)


VOLL = [
    {"feld_id": "rentner_anzahl_renten", "typ": "int", "regel_id": "p22_anzahl_renten_erhebung"},
    {"feld_id": "rentner_jahresrente", "typ": "cent", "instanz_gruppe": "rente",
     "regel_id": "p22_1_leibrente"},
    {"feld_id": "rentner_renten_art", "typ": "enum", "instanz_gruppe": "rente",
     "regel_id": "p22_1_leibrente"},
    {"feld_id": "spenden_betrag", "typ": "cent", "regel_id": "p10b_spenden"},
]


def test_die_anzahl_bringt_das_gezaehlte_mit(nur_rente):
    """RICHTUNG 2, die am 2026-08-28 gefehlt hat. Stufe 2 wählte das Thema „wie viele Renten" und
    nicht das Thema „Rente" — der enge Katalog trug damit die Zählung und nichts zum Zählen.

    Eine Anzahl ohne die Sache, die sie zählt, ist keine Frage."""
    eng = [f for f in VOLL if f["feld_id"] in ("rentner_anzahl_renten", "spenden_betrag")]
    mit = api_llm._mit_zaehlfeldern(eng, VOLL)

    ids = [f["feld_id"] for f in mit]
    assert "rentner_jahresrente" in ids, (
        "Das Zählfeld steht im Katalog, die Renten selbst nicht. Genau so hatte die Frage nach "
        "1850 Euro kein Ziel und landete auf rentner_anzahl_renten.")
    assert "rentner_renten_art" in ids, "Die Gruppe kommt nur halb mit"
    assert ids[:2] == ["rentner_anzahl_renten", "spenden_betrag"], "Die vorhandenen Felder wanderten"


def test_die_gruppe_bringt_ihre_anzahl_mit(nur_rente):
    """RICHTUNG 1, unverändert — der Fall vom 2026-08-25 („verheiratet, 2 kinder" ohne Zahl). Steht
    hier, weil ein Umbau, der beide Richtungen in eine Funktion legt, die alte mitnehmen kann."""
    eng = [f for f in VOLL if f.get("instanz_gruppe") == "rente"]
    ids = [f["feld_id"] for f in api_llm._mit_zaehlfeldern(eng, VOLL)]

    assert "rentner_anzahl_renten" in ids, "Die Instanzfelder stehen da, die Zahl fehlt"
    assert "spenden_betrag" not in ids, "Eine fremde Regel ist mitgekommen"


def test_ein_nachgelegtes_zaehlfeld_zieht_keine_gruppe_nach(nur_rente):
    """DER TEUERSTE FEHLER, den dieser Umbau machen könnte, und er wäre nicht sichtbar, sondern nur
    teuer: schaukeln sich die beiden Richtungen auf, legt Richtung 1 ein Zählfeld nach und
    Richtung 2 zieht daran die ganze Gruppe hinterher.

    Gemessen an neun echten Läufen ist das der Unterschied zwischen +5 passenden Feldern und
    +34 Kind-Feldern im Katalog einer kinderlosen Rentnerin — und ein aufgeblähter Katalog ist
    genau der Zustand, gegen den die drei Stufen gebaut wurden.

    Hier steht EIN Feld der Gruppe im engen Katalog. Richtung 1 muss die Zahl nachlegen; das
    zweite Gruppenfeld darf NICHT mitkommen, denn Stufe 2 hat sein Thema nicht gewählt."""
    eng = [f for f in VOLL if f["feld_id"] == "rentner_jahresrente"]
    ids = [f["feld_id"] for f in api_llm._mit_zaehlfeldern(eng, VOLL)]

    assert "rentner_anzahl_renten" in ids, "Richtung 1 ist beim Umbau verlorengegangen"
    assert "rentner_renten_art" not in ids, (
        "Das eben nachgelegte Zählfeld hat seine ganze Gruppe nachgezogen. Beide Richtungen müssen "
        "denselben unveränderten Katalog lesen, sonst wächst er über sich selbst.")


def test_ohne_paarung_bleibt_der_katalog_gleich(nur_rente):
    eng = [f for f in VOLL if f["feld_id"] == "spenden_betrag"]
    assert api_llm._mit_zaehlfeldern(eng, VOLL) == eng


def test_der_rentnerfall_gegen_die_echte_bindung():
    """Derselbe Beweis, aber gegen die WIRKLICHEN Bindungsdateien statt gegen eine Attrappe — mit
    genau den Regeln, die Stufe 2 im Live-Lauf gewählt hat (aus dem Mitschnitt).

    Eine Attrappe prüft die Funktion; erst die echte Bindung prüft, dass die Paarung
    `rente` → `rentner_anzahl_renten` dort auch wirklich steht."""
    import store as ST
    bindung = TR.lade_bindung()
    katalog = ST.lade_katalog(bindung)
    prompt_katalog = [{"feld_id": fid, "typ": b.get("typ"),
                       "regel_id": (b.get("quelle") or {}).get("regel_id"),
                       "instanz_gruppe": b.get("instanz_gruppe")}
                      for fid, b in bindung.items() if fid in katalog["llm"]]
    je_regel = api_llm._felder_je_regel(prompt_katalog)
    getroffen = ["p10b_spenden", "p10b_spenden_vorhanden", "p22_anzahl_renten_erhebung",
                 "p24a_altersentlastungsbetrag", "p2_einkunftsart_sonstige", "p33_1_2_agb_abzug",
                 "p33_2a_fahrtkostenpauschale", "p33b_behinderten_pauschbetrag",
                 "p33b_behinderung_pflege_vorhanden"]
    kat3 = [f for r in getroffen if r in je_regel for f in je_regel[r]] + je_regel.get("", [])

    ids = {f["feld_id"] for f in api_llm._mit_zaehlfeldern(kat3, prompt_katalog)}

    assert "rentner_anzahl_renten" in ids, "Die Voraussetzung des Falls stimmt nicht mehr"
    assert "rentner_jahresrente" in ids, (
        "Mit genau den Regeln des Live-Laufs fehlt der Rentenbetrag im Katalog weiterhin. Dann hat "
        "die Nutzerin ihre einzige Einkunft genannt und wird nie danach gefragt.")


# ================================================================ durch den ganzen Aufruf

class _Antwort:
    def __init__(self, roh: bytes):
        self._roh = roh

    def read(self):
        return self._roh

    def __enter__(self):
        return self

    def __exit__(self, *a):
        return False


def _stufen(monkeypatch, *, s1, s2, s3):
    """urlopen ersetzen, Antwort NACH SCHEMA-NAMEN — dieselbe Attrappe wie in
    tests/test_rueckfragen_gebuendelt.py."""
    nach_name = {"aussagen": s1, "zuordnung": s2, "dialog": s3}

    def _urlopen(req, timeout=None):
        body = json.loads(req.data.decode())
        name = body.get("response_format", {}).get("json_schema", {}).get("name", "")
        return _Antwort(json.dumps({"provider": "TestAnbieter",
                                    "choices": [{"finish_reason": "stop",
                                                 "message": {"content": json.dumps(
                                                     nach_name.get(name) or {})}}]}).encode())

    monkeypatch.setattr(LC.urllib.request, "urlopen", _urlopen)
    monkeypatch.setattr(LC.time, "sleep", lambda s: None)


@pytest.fixture
def konfiguriert(monkeypatch, tmp_path):
    monkeypatch.setenv("LLM_API_KEY", "test-schluessel-nicht-echt")
    monkeypatch.setenv("LLM_API_BASE", "https://example.test/v1")
    monkeypatch.setenv("LLM_MODEL", "test/modell")
    monkeypatch.setattr(AUDIT, "AUDIT_DIR", str(tmp_path))
    return tmp_path


# Ein Katalog mit `regel_id`, damit Stufe 2 überhaupt etwas zu verengen hat.
KAT_LAUF = [dict(f, regel_id="p22_rente") for f in KAT]
TEXT = "ich bin seit 2023 in rente, bekomme 1850 euro im monat"
S1 = {"aussagen": [{"text": "Der Nutzer ist seit 2023 in Rente.", "beleg": "seit 2023 in rente"},
                   {"text": "Der Nutzer bekommt 1850 Euro im Monat.",
                    "beleg": "bekomme 1850 euro im monat"}]}
S2 = {"zuordnungen": [{"aussage": 0, "regeln": ["p22_rente"]},
                      {"aussage": 1, "regeln": ["p22_rente"]}]}


def test_die_falsch_gebundene_rueckfrage_kommt_ohne_feld_an(konfiguriert, monkeypatch):
    """Der ganze Weg, nicht nur die Funktion: die Prüfung muss im verdrahteten Aufruf mitlaufen."""
    _stufen(monkeypatch, s1=S1, s2=S2, s3={
        "vorschlaege": [], "antwort": "", "unsicher": False,
        "rueckfragen": [_rf(GEMESSEN, "rentner_anzahl_renten", 1)]})

    erg = api_llm._llm_dialog(TEXT, KAT_LAUF, user_id="prüfer")

    assert len(erg["rueckfragen"]) == 1, "Die Frage ist verschwunden statt entkoppelt"
    assert erg["rueckfragen"][0]["feld_id"] == "", (
        "Im echten Aufruf zeigt die Geldfrage weiter auf das Zählfeld — die Prüfung ist nicht "
        "verdrahtet.")


def test_eine_falsch_gebundene_rueckfrage_verdraengt_keinen_vorschlag(konfiguriert, monkeypatch):
    """`_rueckfrage_verdraengt` entfernt zu jedem GEFRAGTEN Feld den Vorschlag — zu Recht: fragen
    und gleichzeitig raten war der Befund vom 2026-08-21. Zeigt die Frage aber auf ein Feld, nach
    dem sie gar nicht fragt, verdrängte sie einen Vorschlag, der mit ihr nichts zu tun hat.

    Deshalb steht die Bindungsprüfung VOR dem Verdrängen. Hier gibt es einen belegten Vorschlag
    für die Zahl der Renten UND eine Geldfrage, die fälschlich auf dasselbe Feld zeigt."""
    _stufen(monkeypatch, s1=S1, s2=S2, s3={
        "vorschlaege": [{"feld_id": "rentner_anzahl_renten", "wert": 1,
                         "beleg": "seit 2023 in rente", "begruendung": "egal", "aussage": 0}],
        "rueckfragen": [_rf(GEMESSEN, "rentner_anzahl_renten", 1)],
        "antwort": "", "unsicher": False})

    erg = api_llm._llm_dialog(TEXT, KAT_LAUF, user_id="prüfer")

    assert [v["feld_id"] for v in erg["vorschlaege"]] == ["rentner_anzahl_renten"], (
        "Der Vorschlag wurde von einer Rückfrage verdrängt, die auf ein anderes Thema zielt. "
        "Reihenfolge in _llm_dialog prüfen: erst die Bindung lösen, dann verdrängen.")


def test_eine_richtig_gebundene_rueckfrage_verdraengt_weiterhin(konfiguriert, monkeypatch):
    """Die Gegenprobe: passt die Frage zum Feld, MUSS sie den Vorschlag dazu weiterhin verdrängen.
    Ohne diesen Test wäre „löse einfach jede Bindung" eine bestandene Lösung — und die Vermutung
    stünde wieder neben der offenen Frage im Fall."""
    _stufen(monkeypatch, s1=S1, s2=S2, s3={
        "vorschlaege": [{"feld_id": "rentner_jahresrente", "wert": 2220000,
                         "beleg": "bekomme 1850 euro im monat", "begruendung": "egal",
                         "aussage": 1}],
        "rueckfragen": [_rf("Wie viel Rente hast du dieses Jahr insgesamt bekommen?",
                            "rentner_jahresrente", 1)],
        "antwort": "", "unsicher": False})

    erg = api_llm._llm_dialog(TEXT, KAT_LAUF, user_id="prüfer")

    assert erg["vorschlaege"] == [], (
        "Die Rückfrage passt zu ihrem Feld und muss den Vorschlag dazu verdrängen — sonst steht "
        "die Vermutung im Fall, während die Frage danach noch offen ist.")
    assert erg["rueckfragen"][0]["feld_id"] == "rentner_jahresrente"


# ================================================================ der Ausfall im Fluss

def test_eine_ausgefallene_stufe_steht_im_fluss(konfiguriert, monkeypatch):
    """ANLASS: `serie-verheiratet-1kind-handwerker-1787909637`. Stufe 1 las fünf Aussagen, danach
    kam nichts mehr. Im Fluss-Mitschnitt fehlten Stufe 2 und 3 einfach; der Grund
    (`grund=abgeschnitten`, also die Längengrenze des Anbieters) stand nur im Audit-Protokoll und
    musste über Zeitstempel danebengelegt werden.

    Genau das Zusammenlegen von Hand soll der Fluss abschaffen. Ein Strang, in dem nur die
    geglückten Schritte stehen, ist kein Fluss, sondern eine Erfolgsmeldung."""
    monkeypatch.setenv("TAXGRAPH_FLOW", "1")

    def stub(rolle, messages, fixture_id=None, schema=None):
        if (schema or {}).get("name") == "aussagen":
            return LC.Completion(text=json.dumps(S1), provider="TestAnbieter", finish="stop")
        raise LC._mit_grund(LC.LlmNichtVerfuegbar("abgeschnitten"), LC.GRUND_ABGESCHNITTEN)
    monkeypatch.setattr(LC, "complete", stub)

    api_llm._llm_dialog(TEXT, KAT_LAUF, user_id="prüfer")

    pfad = os.path.join(str(konfiguriert), "flow.jsonl")
    assert os.path.exists(pfad), "Der Fluss-Mitschnitt fehlt ganz"
    zeilen = [json.loads(z) for z in open(pfad, encoding="utf-8") if z.strip()]
    ki = [z["inhalt"] for z in zeilen if z.get("art") == "ki"]

    ausgefallen = [e for e in ki if e.get("was") == "ausgefallen"]
    assert ausgefallen, (
        f"Der Ausfall steht nicht im Fluss — dort fehlt Stufe 2 einfach, und wer nur den Fluss "
        f"liest, sieht DASS nichts kam und nicht warum. Vorhanden: {[e.get('was') for e in ki]}")
    e = ausgefallen[0]
    assert e["stufe"] == 2, f"Die Stufe fehlt oder ist falsch: {e}"
    assert e["inhalt"]["grund"] == "abgeschnitten", (
        f"Ohne den Grund stünde dort wieder nur, dass der Aufruf schiefging: {e['inhalt']}")


def test_ohne_schalter_bleibt_auch_der_ausfall_stumm(konfiguriert, monkeypatch):
    """Der Fluss führt Klartext und ist standardmässig AUS (produkt/haut/flow.py). Ein neuer
    Schreiber darf daran nicht vorbeischreiben — sonst legte ausgerechnet der Fehlerfall die erste
    Datei an."""
    monkeypatch.delenv("TAXGRAPH_FLOW", raising=False)
    monkeypatch.delenv("TAXGRAPH_KI_DEBUG", raising=False)

    def stub(rolle, messages, fixture_id=None, schema=None):
        raise LC._mit_grund(LC.LlmNichtVerfuegbar("abgeschnitten"), LC.GRUND_ABGESCHNITTEN)
    monkeypatch.setattr(LC, "complete", stub)

    with pytest.raises(LC.LlmNichtVerfuegbar):
        api_llm._llm_dialog(TEXT, KAT_LAUF, user_id="prüfer")

    assert not os.path.exists(os.path.join(str(konfiguriert), "flow.jsonl")), (
        "Ohne Schalter darf kein Mitschnitt entstehen — auch nicht der eines Ausfalls.")
