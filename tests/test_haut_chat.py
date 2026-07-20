"""K1 Chat-Berater — HAUT-Zone (api.chat-Handler) + Ring-e2e (dev-1).

Ergänzt dev-2s Store-Goldens S1-S8 (test_ui_zwei_signal_sicherheit) um die HAUT-Ebene: den /chat-Handler
selbst. Der LLM SCHLÄGT Feld-Werte VOR, setzt NIE einen — jede Sicherheits-Untergrenze liegt strukturell
im Store (Auflage A + Feld-Katalog); dieser Test beweist, dass der HANDLER sie korrekt bedient.

KEIN echter LLM-Call: llm_client.vorschlaege wird monkeypatcht (Fixture-Antwort). Der Live-Call ist
Julius-Cap-gated ($0 ohne $LLM_API_KEY) — genau das prüft test_cap_gate_kein_key_501 (ECHTER llm_client,
kein Mock, kein Netz). Julius-Regel „nie Mock-LLM außer explizit verlangt": hier wird NUR die Handler-
Verdrahtung getestet, die KI-Antwort ist eine injizierte Fixture, nie ein simulierter Call.

Deckt:
  - test_cap_gate_kein_key_501 ...... kein Key → 501 CHAT_501 ($0, kein Call) — die echte llm_client-Grenze.
  - test_prompt_katalog_ist_liste .... Prompt/Check-Katalog-SPLIT (dev-2 msg 4365): der PROMPT-Katalog ist eine
                                       LISTE von Feld-Metadaten (Haut), NICHT der Store-Check-Katalog
                                       {typ→frozenset}. Die frühere Verwechslung hätte /chat immer auf 501 gedrückt.
  - test_happy_path_vorlaeufig ....... zwei erlaubte Felder → beide als VORLÄUFIGE llm:chat-Events (signal_2=null).
  - test_graceful_skip_human_only .... (Instructor-Auflage) ein human-only-Vorschlag (veranlagung) → Store-
                                       ValueError → Handler überspringt STILL, crasht NICHT, schreibt das Feld
                                       NICHT, verarbeitet den Rest weiter.
  - test_ring_e2e_vorlaeufig_bewegt_steuer_nicht ... der echte Ring: ein vorläufiger Chat-Vorschlag bewegt
                                       /ergebnis NICHT; erst der menschliche Confirm senkt die Steuer.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "golden"):
    sys.path.insert(0, os.path.join(ROOT, _sub))
sys.path.insert(0, os.path.join(ROOT, "produkt", "store"))

import api as API          # noqa: E402
import llm_client as LC     # noqa: E402


def _catala_da() -> bool:
    try:
        import runner  # noqa: F401
        return True
    except Exception:
        return False


def _laie(fld, w):
    """Ein menschlich BESTÄTIGTES Event (signal_2 gesetzt) — der Zwei-Signal-Confirm."""
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


@pytest.fixture
def fall(tmp_path, monkeypatch):
    """Ein leerer gesamt-Fall in einem tmp-FAELLE-Verzeichnis (kein Zugriff auf echte Fälle)."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path))
    st, _ = API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "chat1"})
    assert st == 201
    return "chat1"


def _fake_vorschlaege(*eintraege):
    """Baut eine llm_client.vorschlaege-Ersatzfunktion, die eine feste Liste liefert (kein echter Call)."""
    liste = [{"feld_id": fid, "wert": w, "begruendung": "fixture"} for fid, w in eintraege]

    def fake(freitext, katalog):
        fake.gesehener_katalog = katalog          # für den Split-Test festhalten
        return list(liste)
    fake.gesehener_katalog = None
    return fake


# --------------------------------------------------------------- Cap-Gate: kein Key → 501, $0, kein Call
def test_cap_gate_kein_key_501(fall, monkeypatch):
    """Ohne $LLM_API_KEY läuft der ECHTE llm_client (kein Monkeypatch): _key() wirft LlmNichtVerfuegbar
    VOR jedem Netz-Zugriff → der Handler fällt sauber auf 501 CHAT_501 zurück. $0, kein Mock, kein Call."""
    monkeypatch.delenv("LLM_API_KEY", raising=False)
    st, body = API.chat(fall, {"text": "Ich hatte 3000 Euro Krankheitskosten."})
    assert st == 501
    assert body["fehler"] == "not_implemented"     # der Erklär-Vertrag, keine Fake-200


# --------------------------------------------------------------- Prompt/Check-Katalog-Split (dev-2 msg 4365)
def test_prompt_katalog_ist_liste(fall, monkeypatch):
    """Der an llm_client übergebene PROMPT-Katalog ist eine LISTE von Feld-Metadaten (feld_id/fragetext_laie/
    typ/…), NICHT der Store-Check-Katalog {typ→frozenset}. (Die frühere Verwechslung hätte llm_client._prompt
    über die dict-KEYS iterieren lassen → TypeError → stilles 501, Chat nie funktionsfähig.)"""
    spy = _fake_vorschlaege(("agb_aufwendungen", 300000))
    monkeypatch.setattr(LC, "vorschlaege", spy)
    st, _ = API.chat(fall, {"text": "3000 Euro Krankheitskosten."})
    assert st == 200
    kat = spy.gesehener_katalog
    assert isinstance(kat, list) and kat, "Prompt-Katalog muss eine nicht-leere Liste sein"
    assert all(isinstance(e, dict) and "feld_id" in e for e in kat), "jede Zeile = Feld-Metadaten-dict"
    fids = {e["feld_id"] for e in kat}
    assert "agb_aufwendungen" in fids               # ein gesamt-Scheibe-llm-Feld wird angeboten
    assert "veranlagung" not in fids                # human-only wird der KI GAR NICHT erst angeboten


# --------------------------------------------------------------- Happy-Path: zwei erlaubte Felder → vorläufig
def test_happy_path_vorlaeufig(fall, monkeypatch):
    """Die KI schlägt zwei Katalog-erlaubte Felder vor → der Handler schreibt BEIDE als VORLÄUFIGE llm:chat-
    Events (herkunft=llm_vorschlag, signal_2=null). Nichts bewegt die Summe ohne menschlichen Confirm."""
    monkeypatch.setattr(LC, "vorschlaege",
                        _fake_vorschlaege(("agb_aufwendungen", 300000), ("berufsausbildung_aufwendungen", 90000)))
    st, body = API.chat(fall, {"text": "3000 Euro Krankheitskosten, 900 Euro Fortbildung."})
    assert st == 200
    geschrieben = {g["feld_id"] for g in body["vorschlaege"]}
    assert geschrieben == {"agb_aufwendungen", "berufsausbildung_aufwendungen"}
    assert body["abgelehnt"] == []
    # Store-Wahrheit: beide Felder aktiv, vorläufig, llm_vorschlag, kein signal_2
    store = API.lade_fall(fall)
    import store as ST
    aktiv = ST._aktives(store)
    for fid in ("agb_aufwendungen", "berufsausbildung_aufwendungen"):
        ev = aktiv[fid]
        assert ev["zustand"] == "vorlaeufig"
        assert ev["herkunft"]["herkunft"] == "llm_vorschlag"
        assert ev["schreiber"] == "llm:chat"
        assert ev["signal"]["signal_2"] is None


# --------------------------------------------------------------- Graceful-Skip: human-only-Vorschlag (Instructor)
def test_graceful_skip_human_only(fall, monkeypatch, capsys):
    """Instructor-Auflage: schlägt die KI ein HUMAN-ONLY-Feld (veranlagung, Wahlrecht §26) vor, wirft der
    globale Store-Katalog-Check ValueError → der Handler fängt es, überspringt STILL, CRASHT NICHT, schreibt
    das Feld NICHT und verarbeitet die anderen Vorschläge weiter. veranlagung erscheint in `abgelehnt`."""
    monkeypatch.setattr(LC, "vorschlaege", _fake_vorschlaege(
        ("agb_aufwendungen", 300000),
        ("veranlagung", "zusammen"),                # human-only → muss abgewiesen werden
        ("berufsausbildung_aufwendungen", 90000)))
    st, body = API.chat(fall, {"text": "…"})        # KEIN Crash trotz eines bösen Vorschlags
    assert st == 200
    assert {g["feld_id"] for g in body["vorschlaege"]} == {"agb_aufwendungen", "berufsausbildung_aufwendungen"}
    assert body["abgelehnt"] == ["veranlagung"]
    # veranlagung wurde NICHT geschrieben (kein aktives Event)
    import store as ST
    assert "veranlagung" not in ST._aktives(API.lade_fall(fall))
    # Observability PII-frei: nur die abgewiesene feld_id, kein Wert/Freitext
    err = capsys.readouterr().err
    assert "veranlagung" in err and "zusammen" not in err


# --------------------------------------------------------------- Ring-e2e: vorläufig bewegt die Steuer NICHT
@pytest.mark.skipif(not _catala_da(), reason="Catala-Toolchain nicht verfügbar — Ring-Zahl übersprungen")
def test_ring_e2e_vorlaeufig_bewegt_steuer_nicht(fall, monkeypatch):
    """DUAL-GOLDEN (Instructor-Auflage) an der ECHTEN Ring-Rechnung: ein vorläufiger Chat-Vorschlag (agB §33)
      - bewegt /ergebnis (FESTGESETZTE Steuer, nur_bestaetigt=True) NICHT — Security-Invariant am Endpunkt;
      - ZEIGT seine potenzielle Wirkung in /stand ([min,max]-Range, nur_bestaetigt=False) — UX-Invariant;
      - senkt die Steuer erst nach dem menschlichen Confirm (Zwei-Signal).
    Fängt BEIDE Regressionen: Filter-Weg am /ergebnis (Security-Regress) UND Filter-Zu am /stand (UX-Regress,
    Range-Kollaps). VZ-robust über Relationen (kein Magic-Cent)."""
    from test_paket_b_e2e_http import _gesamt_kegel   # kanonischer Kegel-Builder (single source of truth)
    # (1) voller Arbeitnehmer-Kegel bestätigt → Basis-Steuer
    for feld, wert in _gesamt_kegel(0, bruttolohn=4000000, kein_vuv=True):
        st, _ = API.event(fall, _laie(feld, wert))
        assert st == 201
    st, erg = API.ergebnis(fall)
    assert erg["grund"] == "bestaetigt" and isinstance(erg["zahl_cent"], int) and erg["zahl_cent"] > 0
    est_basis = erg["zahl_cent"]

    # (2) Chat schlägt agB VORLÄUFIG vor → /ergebnis UNVERÄNDERT (kein signal_2 → nicht in der Bemessung)
    monkeypatch.setattr(LC, "vorschlaege", _fake_vorschlaege(("agb_aufwendungen", 500000)))
    st, _ = API.chat(fall, {"text": "5000 Euro Krankheitskosten."})
    assert st == 200
    _, erg_vor = API.ergebnis(fall)
    assert erg_vor["zahl_cent"] == est_basis, "ein VORLÄUFIGER llm-Vorschlag darf die festgesetzte Steuer NICHT bewegen"

    # (2b) UX-Seite des Dual-Invariants: /stand ZEIGT die vorläufig-agB-Wirkung im Range (nur_bestaetigt=False),
    # während /ergebnis sie ignoriert. Kegel voll bestätigt → keine offene Achse → Punkt-Schätzung, die aber den
    # vorläufigen agB-Abzug einschließt (niedriger als /ergebnis). Ohne diese Assertion bliebe der /stand-Range-
    # Kollaps (den der unbedingte Filter verursachte) ungetestet.
    _, stand_vor = API.stand(fall)
    iv = stand_vor["intervall"]
    assert iv["min_cent"] == iv["max_cent"], "Kegel voll bestätigt → Punkt-Schätzung (keine offene Achse)"
    assert iv["min_cent"] < est_basis, \
        "/stand MUSS die potenzielle agB-Wirkung zeigen (vorläufig, nur_bestaetigt=False) — /ergebnis NICHT"

    # (3) Mensch bestätigt agB (signal_2, ersetzt den vorläufigen Vorschlag) → jetzt senkt der Abzug die Steuer
    import store as ST
    aktiv_agb = ST._aktives(API.lade_fall(fall))["agb_aufwendungen"]["event_id"]
    best = _laie("agb_aufwendungen", 500000)
    best["ersetzt"] = aktiv_agb                     # Zwei-Signal-Confirm ersetzt das vorläufige llm:chat-Event
    st, _ = API.event(fall, best)
    assert st == 201
    _, erg_nach = API.ergebnis(fall)
    assert erg_nach["zahl_cent"] < est_basis, "der BESTÄTIGTE agB-Abzug muss die Steuer senken"
