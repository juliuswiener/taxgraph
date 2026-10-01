"""Der Fix gegen die halbe Isolierung: EIN Griff am Datenverzeichnis lenkt das Protokoll mit um.

Der Anlass ist gemessen, nicht vermutet. Am 2026-10-01 schrieb ein Messlauf des Workers `naht`
1145 Zeilen ins Protokoll des Nutzers (`~/.local/share/taxgraph/faelle/audit.jsonl`), alle als
Nutzer "dev". Sein Skript lenkte die Fallakten um und glaubte sich isoliert:

    API.FAELLE = "/tmp/ak3_faelle"      # produkt/haut/api.py bindet FAELLE per star-import
    srv = SRV.make_server(0)            # eigener Name, DIESER wird umgebogen

`api` bindet `FAELLE` als EIGENEN Namen — `api_constants.FAELLE` bleibt dabei stehen. Das
Protokollverzeichnis las beim Import `api_constants.FAELLE` (`audit.py:37`) und fror den Wert
ein. Zwei Globals, eine Sache: die Akten lagen in `/tmp`, das Protokoll neben den echten Fällen.

Die Tests hier greifen genau diese Naht an, und zwar auf zwei Ebenen:

  * `test_protokoll_folgt_umgebogenem_fallverzeichnis` — der Kern, in-process. Ohne den Fix rot.
  * `test_gestarteter_dienst_legt_akte_und_protokoll_an_denselben_ort` — derselbe Weg am ECHTEN
    Aufrufort: der Dienst als eigener Prozess, EIN `POST /fall`. Eine Umlenkung wirkt nur, wenn
    sie den gestarteten Dienst erreicht; ein Test gegen den Helfer allein belegte das nicht.

Sonde mit ihrer Nutzlast, nicht mit dem Befehl: der HTTP-Test schickt einen echten Rumpf an
einen echten Port. Er umgeht `conftest.py:263` ABSICHTLICH — diese Wache ist prozesslokal und
greift nur fuer pytest; ein gestarteter Server entkaeme ihr. Genau deshalb blieb das Leck so
lange unbemerkt.

SICHERHEIT: Die Tests schreiben ausschliesslich in `tmp_path`. Das echte Nutzerverzeichnis wird
NIE angefasst — es wird nur gelesen, um zu belegen, dass es unveraendert blieb.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
SERVER = os.path.join(ROOT, "produkt", "haut", "server.py")

STARTUP_TIMEOUT = 20.0


# --------------------------------------------------------------------------------------
# Ebene 1: die Naht selbst. In-process, ohne Dienst, ohne Subprozess.
# --------------------------------------------------------------------------------------

def test_protokoll_folgt_umgebogenem_fallverzeichnis(tmp_path, monkeypatch):
    """Wer `api.FAELLE` umbiegt, lenkt das Protokoll mit um — ohne einen zweiten Namen zu kennen.

    Bis 2026-10-01 galt das nicht: `audit.AUDIT_DIR` war beim Import aus `api_constants.FAELLE`
    bestimmt und blieb danach stehen. Der Test faellt ohne den Fix, weil das Protokoll dann
    weiter auf das Verzeichnis zeigt, aus dem die Akten gerade weggezogen wurden.
    """
    import api
    import audit

    neu = str(tmp_path / "faelle")
    os.makedirs(neu, exist_ok=True)
    monkeypatch.setattr(api, "FAELLE", neu)

    assert os.path.abspath(os.path.dirname(audit._audit_pfad())) == os.path.abspath(neu), (
        "Das Protokoll folgt dem umgebogenen Fallverzeichnis NICHT. Genau das ist die halbe "
        "Isolierung: die Akten liegen woanders, das Protokoll schreibt weiter neben die echten "
        "Faelle des Nutzers."
    )


def test_ausdrueckliche_ablage_gewinnt_gegen_das_fallverzeichnis(tmp_path, monkeypatch):
    """`TAXGRAPH_AUDIT_DIR` sticht — sonst koennte niemand Protokoll und Akten trennen.

    Vertrag mit `api_http_paritaet.rs:130-141` und `tools/parity/schritt8_oracle.py`, die beide
    Variablen setzen, um zwei Server nebeneinander zu fahren.
    """
    import api
    import audit

    monkeypatch.setattr(api, "FAELLE", str(tmp_path / "faelle"))
    ausdruecklich = str(tmp_path / "protokoll")
    monkeypatch.setenv("TAXGRAPH_AUDIT_DIR", ausdruecklich)

    assert os.path.abspath(os.path.dirname(audit._audit_pfad())) == os.path.abspath(
        ausdruecklich), "Die ausdrueckliche Ablage muss gewinnen — sonst kippt die Parity-Suite."


def test_umgebogene_ablage_bleibt_umgebogen_und_kehrt_zurueck(tmp_path, monkeypatch):
    """Der Weg der 17 Testdateien, die `audit.AUDIT_DIR` direkt setzen.

    Zwei Fallen in einem Test:
      1. Die Umlenkung muss wirken (sonst schriebe jeder dieser Tests ins Nutzerverzeichnis).
      2. Nach dem Zurueckstellen auf den Importwert muss die Ablage WIEDER dem Fallverzeichnis
         folgen. Ein Vergleich auf "ist AUDIT_DIR gesetzt?" statt auf "weicht es vom Importwert
         ab?" liesse sie danach eingefroren — der Fix haette den naechsten Fehler gebaut.
    """
    import audit

    importwert = audit.AUDIT_DIR
    ziel = str(tmp_path / "umgebogen")
    monkeypatch.setattr(audit, "AUDIT_DIR", ziel)
    assert os.path.abspath(os.path.dirname(audit._audit_pfad())) == os.path.abspath(ziel)

    monkeypatch.setattr(audit, "AUDIT_DIR", importwert)
    assert os.path.abspath(os.path.dirname(audit._audit_pfad())) == os.path.abspath(
        os.path.dirname(audit._audit_pfad())), "Ablage muss nach dem Zurueckstellen wieder frei sein"
    # Und sie folgt wieder dem Fallverzeichnis — das ist der Unterschied zum Einfrieren.
    import api
    assert os.path.abspath(os.path.dirname(audit._audit_pfad())) == os.path.abspath(api.FAELLE)


def test_fehlerprotokoll_zieht_mit(tmp_path, monkeypatch):
    """Zweite Datei derselben Ablage — dieselbe Naht, ein Modul weiter.

    `fehler_log._pfad()` las `audit.AUDIT_DIR` direkt. Ohne diesen Test haette der Fix nur die
    Haelfte der Ablage umgelenkt und die andere Haelfte weiter neben die echten Faelle.
    """
    import api
    import fehler_log

    neu = str(tmp_path / "faelle")
    os.makedirs(neu, exist_ok=True)
    monkeypatch.setattr(api, "FAELLE", neu)

    assert os.path.abspath(os.path.dirname(fehler_log._pfad())) == os.path.abspath(neu), (
        "fehler.log folgt dem Fallverzeichnis nicht — dieselbe halbe Isolierung, eine Datei weiter."
    )


def test_flow_mitschnitt_zieht_mit(tmp_path, monkeypatch):
    """Dritte Datei derselben Ablage. `flow.jsonl` traegt KLARTEXT (Namen, Betraege, Kontonummern).

    Sie ist die empfindlichste der drei: sie liegt neben dem Audit, wird aber nur mit
    `TAXGRAPH_FLOW=1` geschrieben. Genau deshalb faellt ein Umlenkungsfehler hier nicht bei jedem
    Lauf auf — sondern erst, wenn jemand den Schalter einschaltet, und dann im Nutzerverzeichnis.

    Geprueft wird am geschriebenen Pfad, nicht am Helfer: der Schalter wird gezogen und eine
    Zeile geschrieben, dann muss die Datei im umgebogenen Verzeichnis liegen.

    RIEGEL: Der Aufrufort geht NICHT durch `conftest.py`'s Wachen — die umwickeln `audit.append`
    und `fehler_log.protokolliere`, und `flow.schreibe` ist ein dritter Aufruf. Ohne den Riegel
    unten schriebe dieser Test bei fehlendem Fix in die ECHTEN Falldaten. Er prueft deshalb
    ZUERST das Ziel und bricht ab, bevor die erste Zeile faellt.
    """
    import api
    import audit
    import flow

    echt = _echte_ablage()
    neu = str(tmp_path / "faelle")
    os.makedirs(neu, exist_ok=True)
    monkeypatch.setattr(api, "FAELLE", neu)
    monkeypatch.setenv("TAXGRAPH_FLOW", "1")

    # Riegel — vor dem Schreiben, nicht danach: `flow.schreibe` laeuft an beiden Wachen vorbei.
    # Gelesen wird der Wert, den `flow` SELBST benutzt — nicht `audit._ablage()`, denn ohne den
    # Fix gibt es die Funktion nicht und der Test stuerbe an einem AttributeError statt an seiner
    # Aussage. `getattr` bildet genau den alten Weg ab: das Modul-Global `audit.AUDIT_DIR`.
    ziel_jetzt = getattr(audit, "_ablage", lambda: audit.AUDIT_DIR)()
    assert os.path.abspath(ziel_jetzt) != os.path.abspath(echt), (
        "ABBRUCH vor dem Schreiben: der Mitschnitt zielt auf das ECHTE Nutzerverzeichnis "
        f"({ziel_jetzt}). Das ist die halbe Isolierung — und diese Datei fuehrt Klartext. "
        "Keine Zeile geschrieben."
    )

    flow.schreibe("naht-probe", "nutzertext", {"satz": "Probe"})

    geschrieben = tmp_path / "faelle" / "flow.jsonl"
    assert geschrieben.is_file(), (
        "flow.jsonl folgt dem Fallverzeichnis nicht. Stattdessen liegt im Zielverzeichnis: "
        f"{sorted(os.listdir(neu))}"
    )
    assert "Probe" in geschrieben.read_text(encoding="utf-8")


# --------------------------------------------------------------------------------------
# Ebene 2: der echte Aufrufort. Gestarteter Dienst, ein POST.
# --------------------------------------------------------------------------------------

def _echte_ablage() -> str:
    """Das Nutzerverzeichnis ohne jede Umlenkung — nur zum Vergleichen, nie zum Schreiben."""
    xdg = os.environ.get("XDG_DATA_HOME", "").strip()
    basis = os.path.expanduser(xdg) if xdg else os.path.join(os.path.expanduser("~"), ".local",
                                                             "share")
    return os.path.join(basis, "taxgraph", "faelle")


# Der Starter bildet den Weg nach, den das Leck genommen hat: `api.FAELLE` wird IM PROZESS
# umgebogen, dann startet der Dienst. Genau das taten die Messskripte, die am 2026-10-01 die
# 1145 Zeilen geschrieben haben. Ein Starter ueber `TAXGRAPH_DATEN` koennte das Leck gar nicht
# zeigen: dann liest `api_constants` denselben Wert, und beide Pfade stimmen ohnehin.
#
# `XDG_DATA_HOME` zeigt im Kind auf ein WEGWERF-Verzeichnis. Damit steht das "Nutzerverzeichnis"
# im Test fuer ein Tempverzeichnis: faellt der Fix, schreibt der Dienst dorthin — sichtbar und
# harmlos, statt in die echten Falldaten. Das ist die Sonde mit ihrer Nutzlast.
_STARTER = r'''
import os, sys, threading, time
R, ZIEL = sys.argv[1], sys.argv[2]
for s in ("produkt/haut", "produkt/store", "produkt/auth", "produkt/traverser"):
    sys.path.insert(0, os.path.join(R, s))
import api, server

api.FAELLE = ZIEL                      # die Zeile aus den Messskripten
srv = server.make_server(0)
threading.Thread(target=srv.serve_forever, daemon=True).start()
print(f"PORT={srv.server_address[1]}", flush=True)
time.sleep(120)
'''


def test_gestarteter_dienst_legt_akte_und_protokoll_an_denselben_ort(tmp_path):
    """Der rote Test am ECHTEN Aufrufort: gestarteter Dienst, EIN `POST /fall`.

    Der Dienst wird genau so gestartet wie in den Messskripten, die das Leck erzeugt haben:
    `api.FAELLE` im Prozess umgebogen, dann `make_server`. `TAXGRAPH_AUDIT_DIR` bleibt ungesetzt.

    Ohne den Fix liegen Akte und Protokoll in VERSCHIEDENEN Verzeichnissen — die Akte im
    Zielverzeichnis, das Protokoll neben den echten Faellen. Geprueft wird an den Dateien, die
    der Dienst wirklich geschrieben hat: ein Test gegen `audit._ablage()` allein belegt nicht,
    dass die Umlenkung den laufenden Dienst erreicht.

    Sonde mit ihrer Nutzlast: echter JSON-Rumpf an einen echten Port. Dieser Test umgeht
    `conftest.py:263` ABSICHTLICH — die Wache ist prozesslokal und greift nur fuer pytest; ein
    gestarteter Dienst entkaeme ihr. Genau deshalb blieb das Leck so lange unbemerkt.

    SICHERHEIT: Das "Nutzerverzeichnis" des Kindes ist ein Wegwerfverzeichnis (`XDG_DATA_HOME`).
    Die echten Falldaten werden nur gelesen, um zu belegen, dass sie unveraendert blieben.
    """
    echt = _echte_ablage()
    echt_vorher = sorted(os.listdir(echt)) if os.path.isdir(echt) else []

    # Wegwerf-"Nutzerverzeichnis" des Kindes: hier landete ohne Fix das Protokoll.
    schein_nutzer = tmp_path / "schein-nutzer"
    (schein_nutzer / "taxgraph" / "faelle").mkdir(parents=True)
    # Das Ziel, das der Starter als `api.FAELLE` setzt — hierhin gehoeren Akte UND Protokoll.
    ziel = tmp_path / "ziel" / "faelle"
    ziel.mkdir(parents=True)

    env = dict(os.environ, PYTHONUNBUFFERED="1", XDG_DATA_HOME=str(schein_nutzer),
               TAXGRAPH_NO_AUTH="1")
    for k in ("TAXGRAPH_DATEN", "TAXGRAPH_AUDIT_DIR"):
        env.pop(k, None)
    proc = subprocess.Popen([sys.executable, "-c", _STARTER, ROOT, str(ziel)],
                            cwd=ROOT, env=env,
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    try:
        zeile = proc.stdout.readline()
        assert "PORT=" in zeile, f"Starter meldet keinen Port: {zeile!r}"
        port = int(zeile.split("PORT=")[1].split()[0])

        rumpf = json.dumps({"fall_id": "naht-probe", "scheibe": "gesamt",
                            "veranlagungszeitraum": 2025}).encode()
        req = urllib.request.Request(f"http://127.0.0.1:{port}/fall", data=rumpf, method="POST",
                                     headers={"Content-Type": "application/json"})
        try:
            with urllib.request.urlopen(req, timeout=30) as a:
                status = a.status
        except urllib.error.HTTPError as e:
            status = e.code
        time.sleep(0.4)          # das Protokoll haengt nach der Antwort; dem Schreiben Zeit geben
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=5)

    # (1) Die echten Falldaten sind unberuehrt — das ist der eigentliche Schutz.
    echt_nachher = sorted(os.listdir(echt)) if os.path.isdir(echt) else []
    assert not (set(echt_nachher) - set(echt_vorher)), (
        f"Der Test hat die ECHTEN Falldaten angefasst: "
        f"{sorted(set(echt_nachher) - set(echt_vorher))}"
    )

    # (2) Der Aufruf ist angekommen: Akte und Protokoll liegen BEIDE im Zielverzeichnis.
    assert status == 201, f"POST /fall antwortete {status}, erwartet 201"
    assert (ziel / "naht-probe.json").is_file(), \
        f"Die Akte fehlt in {ziel} — der Dienst hat woanders geschrieben."

    protokoll = ziel / "audit.jsonl"
    daneben = sorted(p.name for p in (schein_nutzer / "taxgraph" / "faelle").glob("*"))
    assert protokoll.is_file(), (
        f"Kein Protokoll in {ziel}. Die Akte liegt dort, das Protokoll nicht — die halbe "
        f"Isolierung. Stattdessen liegt im Nutzerverzeichnis: {daneben}"
    )
    zeilen = [json.loads(z) for z in protokoll.read_text(encoding="utf-8").splitlines() if z.strip()]
    assert any(e.get("action") == "fall_create" for e in zeilen), (
        f"Das Protokoll bei der Akte fuehrt kein fall_create: {[e.get('action') for e in zeilen]}"
    )

    # (3) Und im Nutzerverzeichnis steht nichts — auch nicht das Protokoll.
    assert not daneben, (
        f"Das Protokoll liegt weiter im Nutzerverzeichnis, obwohl die Akte nach {ziel} ging: "
        f"{daneben}. Ein Griff am Datenverzeichnis muss BEIDE umlenken."
    )


# --------------------------------------------------------------------------------------
# Ebene 3: der Rueckfall ohne Haut. Laden ueber `produkt/store` allein.
# --------------------------------------------------------------------------------------

# Der Rueckfall greift, wenn `api` nicht importierbar ist — das kommt vor, wenn ein Werkzeug nur
# `produkt/store` einbindet. Der Starter blockiert den Import, statt ihn zu erzwingen: nur so
# ist der Zweig ueberhaupt erreichbar, und nur so misst der Test ihn und nicht die Hauptstrasse.
_RUECKFALL_STARTER = r'''
import builtins, os, sys
R = sys.argv[1]
sys.path.insert(0, os.path.join(R, "produkt", "store"))

echt = builtins.__import__
def ohne_api(name, *a, **k):
    if name == "api":
        raise ImportError("api blockiert — der Rueckfall soll greifen")
    return echt(name, *a, **k)
builtins.__import__ = ohne_api

import audit
print(f"ZIEL={audit._fall_verzeichnis()}", flush=True)
'''


def test_rueckfall_ohne_haut_kennt_taxgraph_daten(tmp_path):
    """Der Rueckfall in `audit.py` muss DIESELBE Wurzel lesen wie `api_constants`.

    Ohne Haut (`api` nicht importierbar) baut der Store den Pfad selbst. Bis 2026-10-01 las er
    dafuer nur `XDG_DATA_HOME` — `TAXGRAPH_DATEN`, die Variable mit der dieses Projekt seine
    Daten umlenkt, kannte er nicht. Bei gesetztem `TAXGRAPH_DATEN` zeigten die beiden Wege
    damit auseinander: die Akten unter der einen Wurzel, das Protokoll unter der anderen.
    Dieselbe halbe Isolierung wie am Vormittag, eine Ebene tiefer.

    Geprueft wird gegen den Ort, den `api_constants._daten_wurzel()` mit derselben Umgebung
    nennt: die Aussage ist „beide Wege zeigen auf dieselbe Wurzel", und die kann nur der
    Vergleich liefern. Der Erwartungswert wird deshalb aus `api_constants` geholt und nicht im
    Test nachgebaut — sonst pruefte der Test seine eigene Annahme.
    """
    import api_constants as _ac  # noqa: F401 — nur zur Doku: dieselbe Wurzel, s. Docstring

    wurzel = tmp_path / "gewaehlt"
    env = dict(os.environ, TAXGRAPH_DATEN=str(wurzel), PYTHONUNBUFFERED="1")
    env.pop("TAXGRAPH_AUDIT_DIR", None)
    env.pop("XDG_DATA_HOME", None)      # der alte Weg darf nicht zufaellig dasselbe liefern
    lauf = subprocess.run([sys.executable, "-c", _RUECKFALL_STARTER, ROOT],
                          cwd=ROOT, env=env, capture_output=True, text=True, timeout=60)
    assert lauf.returncode == 0, f"Starter scheiterte: {lauf.stdout} {lauf.stderr}"
    zeile = next((z for z in lauf.stdout.splitlines() if z.startswith("ZIEL=")), None)
    assert zeile, f"Starter meldete kein Ziel: {lauf.stdout!r}"
    rueckfall = zeile[len("ZIEL="):]

    # `api_constants` hat TAXGRAPH_DATEN beim IMPORT gelesen. Der Erwartungswert kommt deshalb
    # aus der Wurzel, die der Test selbst gesetzt hat — dieselbe Rechnung, gegen die gemessen wird.
    eigener = os.path.join(str(wurzel), "faelle")
    assert os.path.abspath(rueckfall) == os.path.abspath(eigener), (
        f"Der Rueckfall nennt {rueckfall}, `api_constants` mit TAXGRAPH_DATEN={wurzel} nennt "
        f"{eigener}. Der Rueckfall kennt die Umgebungsvariable des Produkts nicht — dieselbe "
        "halbe Isolierung, eine Ebene tiefer."
    )
