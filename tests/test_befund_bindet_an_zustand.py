"""Invariante 5 ist verdrahtet: ein ERiC-Befund gilt nachweislich für EINEN Datenstand.

produkt/store/SCHEMA.md sagt seit jeher zu:

    Content-adressierte Snapshots — deterministische Materialisierung eines Log-Präfixes,
    adressiert per snapshot_id = sha256(felder). Der ERiC-Befund bindet an genau diesen
    Hash → eine Prüfung gilt nachweislich für EINEN Zustand.

`erzeuge_snapshot()` existierte dafür seit jeher — mit NULL Produktionsaufrufern (Audit
2026-08-16). Die Zusage stand also nur auf dem Papier. Der Nutzer bekam zwar `basis_snapshot`
in der Antwort, aber nichts hielt fest, WELCHER Zustand geprüft worden war: ändert er danach
ein Feld, war nicht feststellbar, dass ein früherer Befund nicht mehr gilt.

Das ist dieselbe Klasse wie das Beleg-Gate, das nie als VERDRAHTET geprüft war, und wie das
Login-Backend, das monatelang fertig dastand, während die Oberfläche nie ein Token schickte:
gebaut, dokumentiert, nie angeschlossen. Ein Test hätte es jederzeit gezeigt — es gab keinen.

WAS HIER GEPRÜFT WIRD, ist deshalb nicht die Snapshot-Mechanik (die hat test_store.py), sondern
die NAHT: dass der Endpunkt sie benutzt, dass der Befund am geprüften Zustand hängt und nicht
an irgendeinem, und dass eine spätere Änderung den alten Befund erkennbar veralten lässt.

NULL LLM, kein ERiC nötig: checkest_gate.validate ist ersetzt — geprüft wird die Bindung,
nicht das Urteil.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/unsicherheit",
             "produkt/mapping", "produkt/konsistenz", "produkt/import", "produkt/bescheid",
             "golden", "elster"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import api as API      # noqa: E402
import audit           # noqa: E402
import store as ST     # noqa: E402


def _aktives_event_id(fall_id: str, feld_id: str) -> str:
    """Die event_id des aktuell aktiven Events zu diesem Feld.

    Auflage B (fail-closed): ein zweiter Wert auf demselben Feld MUSS das vorige ausdrücklich
    ersetzen, sonst lehnt der Store ihn ab. Das ist kein Testdetail, sondern der Grund, warum
    der Log append-only bleiben kann — und weshalb eine Korrektur den Snapshot-Hash bewegt."""
    aktiv = ST._aktives(API.lade_fall(fall_id))
    return aktiv[feld_id]["event_id"]


def _event_rumpf(feld_id: str, wert: int, ersetzt: str | None = None) -> dict:
    """Der Rumpf, den POST /event verlangt (Auflage A: Schreiber und Herkunft müssen sich
    ehrlich deklarieren). Hier bewusst ein bestätigter Laien-Wert mit zweitem Signal — ein
    vorläufiger Wert würde den Fall nie einreichbar machen."""
    rumpf = {"feld_id": feld_id, "wert": wert, "zustand": "bestaetigt",
             "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
             "schreiber": "ui:laie",
             "signal": {"signal_1": None, "signal_2": f"klick@{feld_id}"}}
    if ersetzt:
        rumpf["ersetzt"] = ersetzt
    return rumpf


@pytest.fixture
def fall(tmp_path, monkeypatch):
    """Ein Fall MIT Events — ohne sie gibt es keinen Zustand, an den ein Befund binden könnte
    (der Endpunkt meldet das dann als befund_gebunden=False)."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    monkeypatch.setenv("TAXGRAPH_NO_AUTH", "1")
    _st, r = API.fall_anlegen({"fall_id": "befund1", "scheibe": "gesamt",
                               "veranlagungszeitraum": 2025})
    fid = r["fall_id"]
    st, _ = API.event(fid, _event_rumpf("bruttoarbeitslohn", 5000000))
    assert st in (200, 201), f"Event nicht angelegt: {st}"
    return fid


@pytest.fixture
def eric_ersatz(monkeypatch):
    """checkESt durch eine feste Antwort ersetzen. Der Rückgabewert ist hier gleichgültig —
    geprüft wird, WORAN der Befund hängt, nicht WAS er sagt."""
    import checkest_gate as CE
    import elster_xml as EX
    monkeypatch.setattr(EX, "erzeuge_xml", lambda *a, **k: '<?xml version="1.0"?><Elster/>')
    monkeypatch.setattr(API.EM, "deklariere",
                        lambda *a, **k: {"vollstaendig": True, "deklaration": {"E0100201": "M"},
                                         "unvollstaendig": []})

    def _setze(rc):
        monkeypatch.setattr(CE, "validate", lambda *a, **k: (rc, ""))
    return _setze, CE


def test_befund_wird_am_geprueften_zustand_festgehalten(fall, eric_ersatz):
    """Der Kern. Nach der Prüfung MUSS im Store ein Snapshot liegen, dessen eric_befund an
    genau den Hash gebunden ist, den die Antwort als geprüft nennt."""
    setze, CE = eric_ersatz
    setze(CE.RC_OK)

    status, antwort = API.einreichen(fall, {})
    assert status == 200, antwort
    assert antwort["befund_gebunden"] is True

    store = API.lade_fall(fall)
    snaps = store.get("snapshots") or []
    assert len(snaps) == 1, (
        f"{len(snaps)} Snapshots — die Prüfung hat keinen Zustand festgehalten, die Zusage aus "
        f"SCHEMA.md ist wieder unverdrahtet")
    befund = snaps[0].get("eric_befund")
    assert befund, "Snapshot ohne eric_befund — dann bindet nichts an nichts"
    assert befund["gebunden_an"] == snaps[0]["snapshot_id"], (
        "eric_befund.gebunden_an zeigt nicht auf den eigenen Snapshot")
    assert befund["gebunden_an"] == antwort["basis_snapshot"], (
        f"Der festgehaltene Befund hängt an {befund['gebunden_an'][:12]}, geprüft wurde laut "
        f"Antwort {antwort['basis_snapshot'][:12]} — die Bindung zeigt auf einen anderen "
        f"Zustand als den geprüften, das ist schlimmer als keine Bindung")


def test_auch_ein_roter_befund_wird_gebunden(fall, eric_ersatz):
    """Gerade der rote Fall ist der, den man später einem Datenstand zuordnen will: 'zu welchem
    Stand hat ERiC das beanstandet?'. Würde nur rc=0 gebunden, fehlte die Bindung genau dort,
    wo sie gebraucht wird."""
    setze, CE = eric_ersatz
    setze(CE.RC_PLAUSIBILITAET)

    status, antwort = API.einreichen(fall, {})
    assert status == 422, antwort
    assert antwort["klasse"] == "plausibilitaet_fehler"

    snaps = API.lade_fall(fall).get("snapshots") or []
    assert len(snaps) == 1, "ein Plausibilitätsfehler wurde an keinen Zustand gebunden"
    assert snaps[0]["eric_befund"]["klasse"] == "plausibilitaet_fehler"
    assert snaps[0]["eric_befund"]["gebunden_an"] == antwort["basis_snapshot"]


def test_eine_aenderung_danach_laesst_den_befund_veralten(fall, eric_ersatz):
    """Wofür die Bindung überhaupt da ist. Nach einer Wertänderung materialisiert der Store zu
    einem ANDEREN Hash — der alte Befund zeigt weiterhin auf den alten, und genau daran ist
    erkennbar, dass er für den jetzigen Stand nichts mehr aussagt.

    Ohne die Bindung wäre die Antwort von vorhin ununterscheidbar von einer über den neuen
    Stand: 'ERiC hat das geprüft' — nur eben einen anderen Datenstand."""
    setze, CE = eric_ersatz
    setze(CE.RC_OK)
    _st, antwort = API.einreichen(fall, {})
    geprueft = antwort["basis_snapshot"]

    API.event(fall, _event_rumpf("bruttoarbeitslohn", 6000000,
                                 _aktives_event_id(fall, "bruttoarbeitslohn")))

    store = API.lade_fall(fall)
    _felder, jetzt = ST.materialisiere(store)
    assert jetzt != geprueft, "die Änderung hat den Hash nicht bewegt — dann trägt er nichts"
    alt = store["snapshots"][0]["eric_befund"]["gebunden_an"]
    assert alt == geprueft and alt != jetzt, (
        "Der festgehaltene Befund wandert mit dem Zustand mit — dann bezeugt er nichts")


def test_zweite_pruefung_schreibt_die_historie_fort(fall, eric_ersatz):
    """Mehrfaches Prüfen ist der Normalfall (erst rot, dann korrigiert, dann grün). Jeder
    Durchgang gehört als eigener Eintrag festgehalten, sonst überschreibt der letzte die
    Vorgeschichte — und die ist es, die zeigt, WANN etwas in Ordnung kam."""
    setze, CE = eric_ersatz
    setze(CE.RC_PLAUSIBILITAET)
    API.einreichen(fall, {})

    API.event(fall, _event_rumpf("bruttoarbeitslohn", 5500000,
                                 _aktives_event_id(fall, "bruttoarbeitslohn")))
    setze(CE.RC_OK)
    API.einreichen(fall, {})

    snaps = API.lade_fall(fall).get("snapshots") or []
    assert len(snaps) == 2, f"{len(snaps)} statt 2 Snapshots — die Prüfhistorie geht verloren"
    klassen = [s["eric_befund"]["klasse"] for s in snaps]
    assert klassen == ["plausibilitaet_fehler", "plausibel"], klassen
    assert snaps[0]["eric_befund"]["gebunden_an"] != snaps[1]["eric_befund"]["gebunden_an"], (
        "beide Befunde hängen am selben Hash — dann wurde zweimal derselbe Zustand geprüft, "
        "obwohl dazwischen ein Wert geändert wurde")


def test_bindung_an_einen_fremden_zustand_wird_abgewiesen(fall, eric_ersatz, monkeypatch):
    """Der Hash-Wächter im Endpunkt, direkt geprüft.

    Er fängt einen Zustand, der heute nicht eintreten kann — zwischen der Materialisierung und
    dem Snapshot schreibt niemand. Eine Mutationsprobe (Wächter abschalten) blieb deshalb grün:
    er stand da, ohne dass irgendetwas seine Wirkung belegte. Genau so entsteht Code, den beim
    nächsten Umbau jemand entfernt, weil 'nichts davon abhängt'.

    Hier wird der unmögliche Fall künstlich erzeugt: erzeuge_snapshot liefert einen anderen
    Hash als den geprüften. Ein Befund, der an einen ANDEREN Zustand bindet als den, über den
    ERiC geurteilt hat, ist schlimmer als gar keine Bindung — er täuscht Gewissheit vor."""
    setze, CE = eric_ersatz
    setze(CE.RC_OK)

    echt = ST.erzeuge_snapshot

    def _verschoben(store, **kw):
        snap = echt(store, **kw)
        snap["snapshot_id"] = "f" * 64        # so, als hätte sich der Zustand dazwischen bewegt
        return snap

    monkeypatch.setattr(API.ST, "erzeuge_snapshot", _verschoben)
    with pytest.raises(API.ApiError) as e:
        API.einreichen(fall, {})
    assert e.value.status == 500, f"Status {e.value.status} statt 500"
    assert "anderen Zustand" in str(e.value)


def test_der_store_bleibt_schemagueltig(fall, eric_ersatz):
    """Der Endpunkt schreibt jetzt in den Store. Was er schreibt, muss dem Schema genügen —
    sonst ist die Datei nach der ersten Prüfung kaputt und der nächste Ladevorgang scheitert."""
    jsonschema = pytest.importorskip("jsonschema")
    import json
    setze, CE = eric_ersatz
    setze(CE.RC_OK)
    API.einreichen(fall, {})

    schema = json.load(open(os.path.join(ROOT, "produkt", "store", "schema.json"),
                            encoding="utf-8"))
    fehler = sorted(jsonschema.Draft202012Validator(schema).iter_errors(API.lade_fall(fall)),
                    key=lambda e: list(e.path))
    assert not fehler, "; ".join(f"{list(e.path)}: {e.message}" for e in fehler[:3])


def test_ohne_events_wird_es_gesagt_statt_verschwiegen(tmp_path, monkeypatch, eric_ersatz):
    """Ein Fall ohne Events hat keinen Zustand, an den zu binden wäre — das Schema verlangt für
    `bis_event` die event_id des letzten enthaltenen Events. In der Produktion ist das
    unerreichbar (eine Deklaration ohne ein einziges Event ist nie vollständig, der Aufruf
    gäbe vorher 409); es entsteht nur, wo Tests `deklariere` mocken.

    Es DARF trotzdem nicht still passieren: `befund_gebunden=False` ist die einzige Stelle, an
    der ein Aufrufer sieht, dass die Zusage für dieses Ergebnis nicht eingelöst wurde."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    monkeypatch.setenv("TAXGRAPH_NO_AUTH", "1")
    setze, CE = eric_ersatz
    setze(CE.RC_OK)
    _st, r = API.fall_anlegen({"fall_id": "leer1", "scheibe": "gesamt",
                               "veranlagungszeitraum": 2025})

    status, antwort = API.einreichen(r["fall_id"], {})
    assert status == 200
    assert antwort["befund_gebunden"] is False, (
        "ein Fall ohne Events meldet eine Bindung, die es nicht geben kann")
    assert not (API.lade_fall(r["fall_id"]).get("snapshots") or []), \
        "Snapshot über einen leeren Log — dann zeigt bis_event ins Nichts"
