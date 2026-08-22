"""Der Wortlaut der KI-Antworten — abschaltbar, und standardmässig AUS.

ANLASS (Julius, 2026-08-23): „bitte die ki antworten fuer das debugging anstaendig loggen". Der
Anlass war konkret. Im Verlauf jenes Tages war weder nachvollziehbar, WELCHE Rückfragen die KI
gestellt hatte noch WAS sie geantwortet hat: das Audit führt nur ihre Anzahl (`rueckfragen=5`,
`antwortlaenge=246`), der Wortlaut existierte ausschliesslich im Browserfenster und war nach einem
Neuladen weg. Eine Diagnose ohne Wortlaut ist Ratearbeit.

WARUM ES NICHT INS AUDIT GEHÖRT UND NICHT IMMER LÄUFT: hier steht der INHALT — also alles, was der
PII-Filter nicht erwischt hat. Personennamen zuallererst; die erkennt er bewusst nicht (siehe die
dokumentierte Grenze in pii_filter.py). Das Audit-Protokoll führt seit jeher ausschliesslich
Metadaten, und tests/test_pii_filter.py erzwingt das. Ein Mitschnitt dort wäre kein Debug-Werkzeug,
sondern ein zweiter Datenspeicher ohne Zweckbindung.

Deshalb: `TAXGRAPH_KI_DEBUG=1` schaltet ein, sonst passiert nichts. Diese Datei prüft beide
Richtungen — dass AUS wirklich AUS ist, ist die wichtigere der beiden.

NULL LLM.
"""
from __future__ import annotations

import json
import os
import stat
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
             "produkt/unsicherheit", "produkt/import", "golden"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api_llm   # noqa: E402
import audit     # noqa: E402

# Was im Mitschnitt stehen darf und was nicht — ein Name ist genau das, was der PII-Filter
# durchlässt, also der Grund für den Schalter.
NAME = "Anna Musterfrau"
AUSSAGEN = [{"text": f"Der Nutzer pflegt {NAME}.", "beleg": f"ich pflege {NAME}",
             "status": "vorschlag", "regeln": ["p33b_pflege_pauschbetrag"]}]


def _pfad(tmp_path) -> str:
    return os.path.join(str(tmp_path), "ki_debug.jsonl")


@pytest.fixture
def umgelenkt(tmp_path, monkeypatch):
    """Ablage nach tmp_path — über audit.AUDIT_DIR, denselben Weg, den der Mitschnitt liest."""
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path))
    monkeypatch.delenv("TAXGRAPH_KI_DEBUG", raising=False)
    return tmp_path


def _mitschnitt(monkeypatch, an: bool):
    """Die innere Funktion aus _llm_dialog ist nicht einzeln erreichbar — der Mitschnitt wird
    deshalb über einen echten Durchlauf ausgelöst, dessen erste Stufe sofort scheitert. Das ist
    Absicht: so misst der Test den VERDRAHTETEN Pfad, nicht eine nachgebaute Kopie."""
    if an:
        monkeypatch.setenv("TAXGRAPH_KI_DEBUG", "1")
    import llm_client

    def stub(rolle, messages, fixture_id=None, schema=None):
        return llm_client.Completion(text=json.dumps({"aussagen": AUSSAGEN}),
                                     provider="TestAnbieter", finish="stop")
    monkeypatch.setattr(llm_client, "complete", stub)


def test_standard_ist_aus(umgelenkt, monkeypatch):
    """DIE WICHTIGERE DER BEIDEN RICHTUNGEN. Wer den Schalter vergisst, darf keinen Klartext auf
    der Platte hinterlassen — auch nicht, wenn er nie hinsieht."""
    _mitschnitt(monkeypatch, an=False)
    try:
        api_llm._llm_dialog(f"ich pflege {NAME}", [], user_id="test")
    except Exception:
        pass
    assert not os.path.exists(_pfad(umgelenkt)), (
        "Ohne TAXGRAPH_KI_DEBUG=1 darf keine Mitschnitt-Datei entstehen. Sie enthält Klartext "
        "aus der Steuererklärung; ein Standard-An wäre ein zweiter Datenspeicher ohne Zweck.")


def test_eingeschaltet_steht_der_wortlaut_drin(umgelenkt, monkeypatch):
    """Und die Gegenrichtung: eingeschaltet muss er WIRKLICH da sein, sonst ist der Schalter
    Zierde und die Diagnose weiterhin Ratearbeit."""
    _mitschnitt(monkeypatch, an=True)
    try:
        api_llm._llm_dialog(f"ich pflege {NAME}", [], user_id="test")
    except Exception:
        pass
    p = _pfad(umgelenkt)
    assert os.path.exists(p), "mit TAXGRAPH_KI_DEBUG=1 fehlt die Datei"
    zeilen = [json.loads(z) for z in open(p, encoding="utf-8") if z.strip()]
    assert zeilen, "Datei ist leer"
    assert any(NAME in json.dumps(z, ensure_ascii=False) for z in zeilen), (
        "der Wortlaut fehlt — genau dafür gibt es den Schalter")
    e = zeilen[0]
    for schluessel in ("ts", "stufe", "was", "inhalt"):
        assert schluessel in e, f"{schluessel} fehlt im Eintrag"


def test_die_datei_ist_nicht_fuer_andere_lesbar(umgelenkt, monkeypatch):
    """0600 wie beim Audit-Log. Die Datei führt den Klartext einer Steuererklärung; die umask
    allein liefert 0644 (gemessen im Audit sec-users-json-world-readable)."""
    _mitschnitt(monkeypatch, an=True)
    try:
        api_llm._llm_dialog(f"ich pflege {NAME}", [], user_id="test")
    except Exception:
        pass
    modus = stat.S_IMODE(os.stat(_pfad(umgelenkt)).st_mode)
    assert not modus & 0o077, f"Mitschnitt ist für andere lesbar: {oct(modus)}"


def test_ein_beliebiger_wert_schaltet_nicht_ein(umgelenkt, monkeypatch):
    """Nur exakt "1". Sonst schaltete ein versehentliches `TAXGRAPH_KI_DEBUG=0` — oder ein leerer
    Wert aus einer .env-Zeile — den Klartext-Mitschnitt ein."""
    import llm_client

    def stub(rolle, messages, fixture_id=None, schema=None):
        return llm_client.Completion(text=json.dumps({"aussagen": AUSSAGEN}))
    monkeypatch.setattr(llm_client, "complete", stub)
    for wert in ("0", "", "true", "ja", "2"):
        monkeypatch.setenv("TAXGRAPH_KI_DEBUG", wert)
        try:
            api_llm._llm_dialog(f"ich pflege {NAME}", [], user_id="test")
        except Exception:
            pass
        assert not os.path.exists(_pfad(umgelenkt)), f"TAXGRAPH_KI_DEBUG={wert!r} hat eingeschaltet"


def test_das_audit_bleibt_metadaten_frei(umgelenkt, monkeypatch):
    """Der Mitschnitt darf das Audit nicht anstecken. Beide Protokolle liegen im selben
    Verzeichnis, und genau deshalb wird hier geprüft, dass sie verschiedene Dinge führen."""
    _mitschnitt(monkeypatch, an=True)
    try:
        api_llm._llm_dialog(f"ich pflege {NAME}", [], user_id="test")
    except Exception:
        pass
    ap = os.path.join(str(umgelenkt), "audit.jsonl")
    if not os.path.exists(ap):
        pytest.skip("kein Audit-Eintrag entstanden — dann prüft dieser Test nichts")
    roh = open(ap, encoding="utf-8").read()
    assert NAME not in roh, "der Name steht im AUDIT — dort gehören nur Metadaten hin"
    assert "ich pflege" not in roh, "Freitext im Audit"
