"""K1 Chat-Berater — HAUT-Zone (api.chat-Handler) + Ring-e2e (dev-1).

Ergänzt dev-2s Store-Goldens S1-S8 (test_ui_zwei_signal_sicherheit) um die HAUT-Ebene: den /chat-Handler
selbst. Der LLM SCHLÄGT Feld-Werte VOR, setzt NIE einen — jede Sicherheits-Untergrenze liegt strukturell
im Store (Auflage A + Feld-Katalog); dieser Test beweist, dass der HANDLER sie korrekt bedient.

KEIN echter LLM-Call: llm_client.complete wird monkeypatcht (Fixture-Antwort, fein-granular — der REALE
_chat_prompt/_chat_parse-Task-Wrapper-Code in api.py läuft mit, nur der Netz-Call ist ersetzt). Der Live-Call
ist Julius-Cap-gated ($0 ohne $LLM_API_KEY) — genau das prüft test_cap_gate_kein_key_501 (ECHTER llm_client,
kein Mock, kein Netz). Julius-Regel „nie Mock-LLM außer explizit verlangt": die KI-Antwort ist eine injizierte
Fixture, nie ein simulierter Call.

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

import json
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
import audit                # noqa: E402


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
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path))
    st, _ = API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "chat1"})
    assert st == 201
    return "chat1"


def _fake_complete(*eintraege):
    """Baut eine llm_client.complete-Ersatzfunktion (Fixture-Antwort als Completion, kein echter Call, kein
    Mock) — der reale _chat_prompt/_chat_parse-Code in api.py läuft mit, nur der Netz-Call ist ersetzt."""
    liste = [{"feld_id": fid, "wert": w, "begruendung": "fixture"} for fid, w in eintraege]

    def fake(role, messages, fixture_id=None):
        fake.gesehene_messages = messages          # für den Split-Test festhalten
        return LC.Completion(text=json.dumps(liste))
    fake.gesehene_messages = None
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
    """Der Chat-Prompt (_chat_prompt in api.py) wird aus einer LISTE von Feld-Metadaten gebaut (feld_id/
    fragetext_laie/typ/…), NICHT aus dem Store-Check-Katalog {typ→frozenset}. (Eine Verwechslung hätte
    _chat_prompt über die dict-KEYS iterieren lassen → TypeError → stilles 501, Chat nie funktionsfähig.)
    Seam: llm_client.complete monkeypatcht (fein-granular, unterhalb von _chat_prompt) — der reale Prompt-Bau
    läuft mit, geprüft über den erzeugten System-Message-Inhalt."""
    spy = _fake_complete(("agb_aufwendungen", 300000))
    monkeypatch.setattr(LC, "complete", spy)
    st, _ = API.chat(fall, {"text": "3000 Euro Krankheitskosten."})
    assert st == 200
    system_msg = spy.gesehene_messages[0]["content"]
    assert "agb_aufwendungen" in system_msg          # ein gesamt-Scheibe-llm-Feld wird angeboten
    # human-only wird der KI GAR NICHT erst angeboten. Beispielfeld ist antrag_ermaessigter_satz
    # (Wahlrecht § 34 Abs. 3) — es liegt in der gesamt-Scheibe und ist askable, die Prüfung ist also
    # nicht vakuos. Früher stand hier veranlagung; das ist seit 2026-08-12 auf Julius' Entscheid
    # llm-freigegeben (s. tests/test_veranlagung_llm_freigabe.py).
    assert "antrag_ermaessigter_satz" not in system_msg


# --------------------------------------------------------------- Happy-Path: zwei erlaubte Felder → vorläufig
def test_happy_path_vorlaeufig(fall, monkeypatch):
    """Die KI schlägt zwei Katalog-erlaubte Felder vor → der Handler schreibt BEIDE als VORLÄUFIGE llm:chat-
    Events (herkunft=llm_vorschlag, signal_2=null). Nichts bewegt die Summe ohne menschlichen Confirm."""
    monkeypatch.setattr(LC, "complete",
                        _fake_complete(("agb_aufwendungen", 300000), ("berufsausbildung_aufwendungen", 90000)))
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
    """Instructor-Auflage: schlägt die KI ein HUMAN-ONLY-Feld (antrag_ermaessigter_satz, Wahlrecht § 34
    Abs. 3) vor, wirft der globale Store-Katalog-Check ValueError → der Handler fängt es, überspringt
    STILL, CRASHT NICHT, schreibt das Feld NICHT und verarbeitet die anderen Vorschläge weiter. Das Feld
    erscheint in `abgelehnt`.

    Beispielfeld war bis 2026-08-12 veranlagung (§ 26); das ist auf Julius' Entscheid llm-freigegeben
    worden und wäre hier jetzt ein KONFLIKT statt einer Ablehnung — siehe
    tests/test_veranlagung_llm_freigabe.py. antrag_ermaessigter_satz ist der gleichwertige Ersatz:
    ebenfalls Wahlrecht, ebenfalls human-only, ebenfalls in der gesamt-Scheibe."""
    monkeypatch.setattr(LC, "complete", _fake_complete(
        ("agb_aufwendungen", 300000),
        ("antrag_ermaessigter_satz", True),         # human-only → muss abgewiesen werden
        ("berufsausbildung_aufwendungen", 90000)))
    st, body = API.chat(fall, {"text": "…"})        # KEIN Crash trotz eines bösen Vorschlags
    assert st == 200
    assert {g["feld_id"] for g in body["vorschlaege"]} == {"agb_aufwendungen", "berufsausbildung_aufwendungen"}
    assert body["abgelehnt"] == ["antrag_ermaessigter_satz"]   # UNVERÄNDERTE Form: list[feld_id]
    # NEU (additiv): der Grund steht jetzt daneben — Katalog-Text, PII-frei (kein Wert/Freitext, nur die feld_id).
    assert "antrag_ermaessigter_satz" in body["abgelehnt_gruende"]
    grund = body["abgelehnt_gruende"]["antrag_ermaessigter_satz"]
    assert "antrag_ermaessigter_satz" in grund and "True" not in grund
    assert body["konflikte"] == []                   # kein Konflikt — das Feld war nie LLM-vorschlagbar (Fall 1)
    # das Feld wurde NICHT geschrieben (kein aktives Event)
    import store as ST
    assert "antrag_ermaessigter_satz" not in ST._aktives(API.lade_fall(fall))
    # Observability PII-frei: nur die abgewiesene feld_id, kein Wert/Freitext
    err = capsys.readouterr().err
    assert "antrag_ermaessigter_satz" in err and "True" not in err


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
    monkeypatch.setattr(LC, "complete", _fake_complete(("agb_aufwendungen", 500000)))
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


# --------------------------------------------------------------- Konfliktdialog (Auftrag team-lead, msg Konfliktdialog)
# Fall (1) = human-only/Katalog/F2 -> abgelehnt (oben, test_graceful_skip_human_only), Fall (2) = Nutzer hat das
# Feld schon selbst gesetzt -> KONFLIKT (b/d/e unten). Auflage B bleibt scharf: NUR der Mensch (schreiber=ui:…)
# darf ein aktives Event ersetzen, nie llm:chat — s. store.py append_event Auflage A + B.

def test_konflikt_bei_bereits_gesetztem_feld(fall, monkeypatch):
    """(b) Die KI schlägt ein Feld vor, das der Nutzer schon SELBST (bestätigt) gesetzt hat: Auflage B verbietet
    dem Store das Überschreiben ohne `ersetzt` — der Handler darf das NICHT mehr still in `abgelehnt` verstecken
    (Fall 1 und Fall 2 waren bisher ununterscheidbar). Stattdessen: ein strukturierter Eintrag in `konflikte`
    (feld_id/aktueller Wert/Vorschlag/Begründung/gross) — nichts wird geschrieben, nichts verschwindet."""
    st, _ = API.event(fall, _laie("agb_aufwendungen", 200000))
    assert st == 201
    monkeypatch.setattr(LC, "complete", _fake_complete(("agb_aufwendungen", 500000)))
    st, body = API.chat(fall, {"text": "Eigentlich waren es 5000 Euro Krankheitskosten."})
    assert st == 200
    assert body["vorschlaege"] == []                 # NICHT geschrieben — weder still noch offen
    assert body["abgelehnt"] == []                    # KEIN stiller Fall-1-Fehlgriff — das ist Fall 2
    assert len(body["konflikte"]) == 1
    k = body["konflikte"][0]
    assert k["feld_id"] == "agb_aufwendungen"
    assert k["aktueller_wert"] == 200000               # der vom Nutzer bestätigte Wert
    assert k["vorschlag_wert"] == 500000                # der KI-Vorschlag
    assert k["begruendung"] == "fixture"
    assert k["gross"] is False                          # agb_aufwendungen steuert keine andere Regel
    # Store-Wahrheit: NICHTS hat sich bewegt — genau das ursprüngliche menschliche Event ist noch aktiv
    import store as ST
    aktiv = ST._aktives(API.lade_fall(fall))["agb_aufwendungen"]
    assert aktiv["wert"] == 200000 and aktiv["schreiber"] == "ui:laie"


def test_uebernahme_konflikt_via_event_menschliches_signal(fall, monkeypatch):
    """(d) Übernahme eines gemeldeten Konflikts: die UI (nicht das LLM!) schickt den bestehenden /event-Pfad mit
    `ersetzt=<konflikt.aktuelles_event_id>` und einem MENSCHLICHEN Signal (schreiber=ui:laie, signal_2 gesetzt) —
    exakt derselbe Zwei-Signal-Confirm-Mechanismus wie jede andere Bestätigung (s. test_ring_e2e_vorlaeufig_
    bewegt_steuer_nicht). Kein neuer Schreibpfad nötig — `aktuelles_event_id` im Konflikt-Objekt macht ihn nur
    für die UI erreichbar. Danach: altes Event inaktiv, neues bestätigt mit dem übernommenen Wert."""
    st, _ = API.event(fall, _laie("agb_aufwendungen", 200000))
    assert st == 201
    monkeypatch.setattr(LC, "complete", _fake_complete(("agb_aufwendungen", 500000)))
    st, body = API.chat(fall, {"text": "Eigentlich waren es 5000 Euro."})
    assert st == 200
    k = body["konflikte"][0]
    alt_event_id = k["aktuelles_event_id"]

    uebernahme = _laie("agb_aufwendungen", k["vorschlag_wert"])
    uebernahme["ersetzt"] = alt_event_id
    st, _ = API.event(fall, uebernahme)
    assert st == 201

    import store as ST
    store = API.lade_fall(fall)
    aktiv = ST._aktives(store)["agb_aufwendungen"]
    assert aktiv["wert"] == 500000
    assert aktiv["zustand"] == "bestaetigt"
    assert aktiv["signal"]["signal_2"] is not None       # MENSCHLICHES Signal, nicht die KI
    assert aktiv["schreiber"] == "ui:laie"                # NICHT llm:chat
    assert aktiv["event_id"] != alt_event_id              # das alte Event ist ersetzt, also inaktiv
    ersetzt_ids = {e["ersetzt"] for e in store["events"] if e.get("ersetzt")}
    assert alt_event_id in ersetzt_ids


def test_llm_bekommt_nie_ersetzt_recht(fall, monkeypatch):
    """(e) Negativtest: das LLM bekommt STRUKTURELL nie ein ersetzt-Recht. Bei einem erkannten Konflikt (Fall 2)
    schreibt chat() KEIN llm:chat-Event — weder vorläufig noch mit `ersetzt`. Prüft den STORE direkt (nicht nur
    die Antwort): unter allen Events mit schreiber=llm:chat darf für agb_aufwendungen keins existieren, und das
    ursprüngliche menschliche Event bleibt unangetastet aktiv."""
    st, _ = API.event(fall, _laie("agb_aufwendungen", 200000))
    assert st == 201
    monkeypatch.setattr(LC, "complete", _fake_complete(("agb_aufwendungen", 500000)))
    st, body = API.chat(fall, {"text": "Eigentlich waren es 5000 Euro."})
    assert st == 200
    assert len(body["konflikte"]) == 1                   # der Konflikt wurde ERKANNT...
    store = API.lade_fall(fall)
    llm_events = [e for e in store["events"] if e["schreiber"] == "llm:chat"]
    assert llm_events == []                               # ...aber NIE geschrieben — kein ersetzt-Versuch, kein Event
    import store as ST
    aktiv = ST._aktives(store)["agb_aufwendungen"]
    assert aktiv["wert"] == 200000 and aktiv["schreiber"] == "ui:laie"   # unangetastet


def test_store_guard_ersetzt_gesperrt_fuer_vorschlags_schreiber():
    """INVARIANTE statt Disziplin (team-lead-Fund Nr. 6, gehärtet 2026-08-12): test_llm_bekommt_nie_ersetzt_
    recht oben beweist nur, dass chat() nie versucht ersetzt zu übergeben — der Store selbst prüfte vorher NUR
    herkunft/zustand/signal_2 für llm:, nicht `ersetzt`. Wer /event direkt mit schreiber=llm:chat + ersetzt=
    aufruft (der Handler event() reicht schreiber/ersetzt roh durch, api.py:2428/2439), käme durch — ein
    UNBESTÄTIGTER Vorschlag würde ein BESTÄTIGTES Nutzer-Event stillschweigend inaktiv setzen (materialisiere()
    nimmt das jüngste nicht-ersetzte Event). Dieser Test prüft ST.append_event DIREKT, unterhalb von chat().
    Gilt für llm:/import:beleg/import:kontoauszug (kein legitimer Aufrufer gemessen). Gegenprobe: berechnet:maps
    bleibt AUSGENOMMEN — entfernung() (api.py, ~L2874-2879) braucht ersetzt= für den Nutzer-getriggerten
    Neuberechnen-Klick, einziger gemessene legitime Fall — kein Über-Blocken."""
    import store as ST
    s = ST.leerer_store(2025)
    mensch = ST.append_event(
        s, feld_id="agb_aufwendungen", wert=200000, zustand="bestaetigt",
        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"})
    katalog = {"llm": frozenset({"agb_aufwendungen"}), "beleg": frozenset({"agb_aufwendungen"}),
               "kontoauszug": frozenset({"agb_aufwendungen"}), "maps": frozenset({"ep_entfernung_km"})}
    for schreiber, herkunft_key in (("llm:chat", "llm_vorschlag"), ("import:beleg", "beleg_import"),
                                     ("import:kontoauszug", "kontoauszug")):
        with pytest.raises(ValueError, match="ersetzt"):
            ST.append_event(
                s, feld_id="agb_aufwendungen", wert=999999, zustand="vorlaeufig",
                herkunft={"herkunft": herkunft_key, "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                schreiber=schreiber, signal={"signal_1": {"typ": "x"}, "signal_2": None},
                ersetzt=mensch["event_id"], katalog=katalog)
    # Gegenprobe: berechnet:maps darf weiterhin ersetzen (entfernung()-Nutzung) — der Guard ist scoped, kein
    # Über-Blocken des einzigen legitimen Falls.
    km_alt = ST.append_event(
        s, feld_id="ep_entfernung_km", wert=10, zustand="vorlaeufig",
        herkunft={"herkunft": "berechnet", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        schreiber="berechnet:maps", signal={"signal_1": {"typ": "maps"}, "signal_2": None}, katalog=katalog)
    km_neu = ST.append_event(
        s, feld_id="ep_entfernung_km", wert=12, zustand="vorlaeufig",
        herkunft={"herkunft": "berechnet", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        schreiber="berechnet:maps", signal={"signal_1": {"typ": "maps"}, "signal_2": None},
        ersetzt=km_alt["event_id"], katalog=katalog)
    assert km_neu["ersetzt"] == km_alt["event_id"]


# --------------------------------------------------------------- "gross"-Kriterium (Julius-Vorschlag, geprüft)
def test_ist_struktureller_konflikt_kriterium():
    """Kriterium für Stufe 2 (Rückfrage statt Häkchen): `gross` = das Feld steuert SELBST eine andere Regel
    (feld_id kommt als `feld` in einer regel_bedingung vor, bindung_regel_bedingungen.yaml). Gemessen: aktuell
    GENAU EIN Eintrag (veranlagung, § 26 Abs. 2 EStG) — Julius' eigenes Beispiel (Einzel- vs. Zusammenveranlagung)
    trifft das Kriterium exakt. WICHTIG (gemessen, nicht geraten): veranlagung trägt selbst KEIN `vorschlagbar_von`
    (human-only per Default-fail-closed, s. store.lade_katalog) — die KI kann es also nie vorschlagen und damit
    nie über chat() in `konflikte` mit gross=True landen; dieser Test prüft das Kriterium deshalb DIREKT, nicht
    end-to-end über chat(). Ein katalog-erlaubtes Feld ohne Regel-Bedingung (agb_aufwendungen) bleibt `gross=False`."""
    assert API._ist_struktureller_konflikt("veranlagung") is True
    assert API._ist_struktureller_konflikt("agb_aufwendungen") is False
