"""Gate: `tools/parity/korpus_faelle.py` endet nach einer Abweisung mit Exit 1 und nennt ihre Zahl.

Vorher zaehlte `main()` nur `neu` und `vorhanden` und gab immer 0 zurueck. Eine Abweisung stand nur in
der Zeile ihres Falls; ein Lauf, der keinen Fall anlegen konnte, endete trotzdem mit Exit 0, und ein
Aufrufer, der nur den Exit-Code liest, sah Erfolg. Jetzt: mindestens eine Abweisung (beim Anlegen oder
bei einem Event) ergibt Exit 1, die Schlusszeile nennt ihre Zahl; `neu` und `vorhanden` (409) bleiben
Exit 0.

Der Lauf geht durch den echten Start-Zweig (`runpy`, `__main__`: `sys.exit(main())`) ueber den echten
Korpus, denn der Exit-Code entsteht erst dort, nicht in `main()`. Kein Netzwerk, kein Server, kein
Schreiben: `API.fall_anlegen` und `API.event` sind ersetzt, jede Abweisung ist ein echtes
`API.ApiError`. Zwei Tests, damit jede Zusage einzeln rot wird: der Exit-Code und die Schlusszeile.
"""
from __future__ import annotations

import runpy
import sys

import pytest

from test_korpus_faelle_abweisung_klartext import GRUND, SKRIPT, _lade  # gleicher Lader wie der Klartext-Test

# (Abweisungen beim Anlegen, Abweisungen bei einem Event, schon vorhandene Faelle). Der Rest des
# Korpus wird neu angelegt. Ein vorhandener Fall (409) ist in jedem Lauf dabei: er darf nie zaehlen.
# Die Event-Abweisung ist der halb angelegte Fall: die Datei steht, das erste Event wurde abgewiesen.
SZENARIEN = [
    pytest.param(0, 0, 1, id="ohne-abweisung"),
    pytest.param(1, 0, 1, id="nur-anlegen"),
    pytest.param(0, 1, 1, id="nur-event"),
    pytest.param(1, 1, 1, id="anlegen-und-event"),
]


def _lauf(monkeypatch, tmp_path, capsys, anlegen_ab, event_ab, vorhanden):
    """Das Skript als Programm laufen lassen. Gibt (Exit-Code, Schlusszeile, Zahl der Faelle) zurueck."""
    kf = _lade(monkeypatch, tmp_path)
    ids = [fall_id for fall_id, _, _ in kf.FAELLE]
    abgewiesen_anlegen = ids[:anlegen_ab]
    abgewiesen_event = ids[anlegen_ab:anlegen_ab + event_ab]
    schon_da = ids[anlegen_ab + event_ab:anlegen_ab + event_ab + vorhanden]

    def anlegen(body):
        if body["fall_id"] in abgewiesen_anlegen:
            raise kf.API.ApiError(400, GRUND)
        if body["fall_id"] in schon_da:
            raise kf.API.ApiError(409, "existiert bereits")
        return 201, {}

    def event(fall_id, body):
        if fall_id in abgewiesen_event:
            raise kf.API.ApiError(422, GRUND)
        return 201, {}

    monkeypatch.setattr(kf.API, "fall_anlegen", anlegen)
    monkeypatch.setattr(kf.API, "event", event)
    monkeypatch.setattr(sys, "argv", ["korpus_faelle.py"])   # argparse liest sonst die pytest-Argumente
    with pytest.raises(SystemExit) as ende:
        runpy.run_path(SKRIPT, run_name="__main__")
    return ende.value.code, capsys.readouterr().out.strip().splitlines()[-1], len(ids)


@pytest.mark.parametrize("anlegen_ab, event_ab, vorhanden", SZENARIEN)
def test_exit_1_genau_dann_wenn_mindestens_eine_abweisung(monkeypatch, tmp_path, capsys,
                                                           anlegen_ab, event_ab, vorhanden):
    code, _, _ = _lauf(monkeypatch, tmp_path, capsys, anlegen_ab, event_ab, vorhanden)
    assert code == (1 if anlegen_ab + event_ab else 0)


@pytest.mark.parametrize("anlegen_ab, event_ab, vorhanden", SZENARIEN)
def test_schlusszeile_nennt_die_zahl_der_abweisungen(monkeypatch, tmp_path, capsys,
                                                     anlegen_ab, event_ab, vorhanden):
    _, zeile, gesamt = _lauf(monkeypatch, tmp_path, capsys, anlegen_ab, event_ab, vorhanden)
    abgewiesen = anlegen_ab + event_ab
    neu = gesamt - abgewiesen - vorhanden     # der Lauf macht nach einer Abweisung weiter
    assert zeile == f"{neu} neu angelegt, {vorhanden} bereits vorhanden, {abgewiesen} abgewiesen, {gesamt} gesamt"
