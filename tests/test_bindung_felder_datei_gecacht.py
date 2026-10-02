"""Die feld_ids einer Bindungsdatei (`felder_datei`) liest ein Prozess einmal — nicht bei jedem Aufruf.

Die Scheibe `n_vor_gwg` führt ihre Felder nicht als Tupel (`felder=None`), sondern über
`felder_datei`. Dafür las `api._scheibe_bindung` (jeder Handler dieser Scheibe) und las das
Parity-Orakel (jede `bescheid.fall`-Anfrage mit dieser Scheibe) bei JEDEM Aufruf die ganze
Bindungsdatei neu — mit PyYAML, das in reinem Python läuft. Jetzt liest sie
`traverser.lade_datei_felder` einmal je Prozess, wie `lade_bindung` daneben.

Der Test zählt die Lesevorgänge, nicht die Sekunden (Muster `test_ui_instanzen.py`): eine
Zeitschranke wäre auf einer langsamen Maschine mal so, mal so, und genau hier soll nichts wackeln.
Er geht durch jede Lesestelle, die vorher selbst gelesen hat — wer eine davon wieder auf
`yaml.safe_load(open(...))` zurückbaut, macht genau ihren Eintrag rot:

  * `api._scheibe_bindung` (Produktion), `bescheid_oracle._scheibe` (Orakel, jede Anfrage),
  * `bescheid_oracle._zweig` und `_scheiben`: mit der echten Konfiguration unerreichbar
    (`n_vor_gwg` hat `gesamt_ring=None`), deshalb mit einem gesetzten Ring in `SCHEIBEN`,
  * `oracle.py` (`traverser.scheiben`): als echter Orakel-Prozess, denn das Modul importiert die
    Catala-Pakete und gehört nicht in den pytest-Prozess.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys

import pytest
import yaml

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
            "produkt/unsicherheit", "golden", "produkt/auth", "tools/parity"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API                # noqa: E402
import bescheid_oracle as BO     # noqa: E402
import traverser as TR           # noqa: E402

BINDUNG = os.path.join(ROOT, "produkt", "bindung")
DATEI = "bindung_n_vor_gwg.yaml"
RING = "nvg_testring"            # nur für _zweig/_scheiben, s. oben
AUFRUFE = 5

EINSTIEGE = {
    "api._scheibe_bindung": lambda: API._scheibe_bindung({"scheibe": "n_vor_gwg"}),
    "bescheid_oracle._scheibe": lambda: BO._scheibe("n_vor_gwg"),
    "bescheid_oracle._zweig": lambda: BO._zweig({"quantitaet": RING, "vz": 2025}),
    "bescheid_oracle._scheiben": lambda: BO._scheiben({}),
}


@pytest.fixture
def lesevorgaenge(monkeypatch):
    """Die Dateinamen, die `yaml.safe_load` gerade liest — in der Reihenfolge der Aufrufe."""
    gelesen: list[str] = []
    echt = yaml.safe_load

    def zaehlt(stream, *a, **k):
        gelesen.append(str(getattr(stream, "name", "")))
        return echt(stream, *a, **k)

    monkeypatch.setattr(yaml, "safe_load", zaehlt)
    return gelesen


@pytest.mark.parametrize("name", EINSTIEGE)
def test_felder_datei_wird_je_prozess_einmal_gelesen(name, monkeypatch, lesevorgaenge):
    BO._module()                     # Imports + lade_bindung() liegen vor dem Zählen
    ac = BO._module()["AC"]
    monkeypatch.setitem(ac.SCHEIBEN["n_vor_gwg"], "gesamt_ring", RING)
    TR.lade_datei_felder.cache_clear()
    lesevorgaenge.clear()
    for _ in range(AUFRUFE):
        EINSTIEGE[name]()
    n = [g for g in lesevorgaenge if g.endswith(DATEI)]
    assert len(n) == 1, (
        f"{name}: {len(n)} Lesevorgänge von {DATEI} bei {AUFRUFE} Aufrufen statt 1 — der Cache "
        f"greift an dieser Stelle nicht, und jeder Aufruf parst die Datei erneut.")


def test_der_cache_liefert_dasselbe_wie_der_frische_read():
    """Ein Cache, der etwas anderes liefert als die Datei, wäre schlimmer als der langsame Weg."""
    dateien = sorted(f for f in os.listdir(BINDUNG) if f.startswith("bindung_") and f.endswith(".yaml"))
    assert DATEI in dateien
    for datei in dateien:
        gecacht = TR.lade_datei_felder(datei)
        assert isinstance(gecacht, tuple), f"{datei}: {type(gecacht).__name__} statt tuple"
        assert gecacht == TR.lade_datei_felder.__wrapped__(datei), datei
    assert len(TR.lade_datei_felder(DATEI)) > 0


_ZAEHLER = '''\
import os
import yaml
_ziel = os.environ["YAML_ZAEHLER"]
_echt = yaml.safe_load
def _zaehlt(stream, *a, **k):
    with open(_ziel, "a") as f:
        f.write(str(getattr(stream, "name", "")) + "\\n")
    return _echt(stream, *a, **k)
yaml.safe_load = _zaehlt
'''


def test_orakel_prozess_liest_die_felder_datei_nicht_je_anfrage(tmp_path):
    """`oracle.py` am echten Aufrufort: der Prozess, den die Rust-Parity-Tests starten. Die
    Lesevorgänge zählt eine `sitecustomize.py` im Kindprozess; sie dürfen mit der Zahl der Anfragen
    nicht wachsen. (Die absolute Zahl bleibt offen — `lade_bindung` & Co. lesen die Datei auch.)"""
    if not os.path.isdir(os.path.join(ROOT, "oracle", "gettsim", "_catala")):
        pytest.skip("Catala-Paket nicht gebaut (make build-python) — oracle.py startet nicht")
    (tmp_path / "sitecustomize.py").write_text(_ZAEHLER, encoding="utf-8")
    zaehler = tmp_path / "zaehler.txt"
    zaehler.write_text("", encoding="utf-8")
    env = dict(os.environ, PYTHONPATH=str(tmp_path), YAML_ZAEHLER=str(zaehler))
    p = subprocess.Popen([sys.executable, os.path.join("tools", "parity", "oracle.py")], cwd=ROOT, env=env,
                         stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)

    def lesen() -> int:
        return sum(1 for z in zaehler.read_text(encoding="utf-8").splitlines() if z.endswith(DATEI))

    def anfragen(n: int) -> dict:
        for _ in range(n):
            p.stdin.write(json.dumps({"fn": "traverser.scheiben"}) + "\n")
            p.stdin.flush()
            antwort = json.loads(p.stdout.readline())
        return antwort

    try:
        antwort = anfragen(2)
        nach_2 = lesen()
        anfragen(AUFRUFE)
        nach_7 = lesen()
    finally:
        p.stdin.close()
        p.wait(timeout=60)
    assert len(antwort["ok"]["n_vor_gwg"]["felder"]) > 0
    assert nach_2 >= 1, "der Zähler sieht keinen einzigen Lesevorgang — die Messung ist blind"
    assert nach_7 == nach_2, (
        f"{nach_7 - nach_2} weitere Lesevorgänge von {DATEI} für {AUFRUFE} weitere Anfragen "
        f"(`traverser.scheiben`) — oracle.py parst die Datei je Anfrage neu.")
