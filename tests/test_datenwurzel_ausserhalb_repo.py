"""Steuerdaten liegen nicht im Projektverzeichnis — und die Sicherung zeigt auf denselben Ort.

DIE ENTSCHEIDUNG (2026-08-19, Audit verschluesselung-steuerdaten-im-klartext): der Fall-Store
zieht aus `produkt/haut/faelle` nach `$XDG_DATA_HOME/taxgraph` bzw. `~/.local/share/taxgraph`.
Verschlüsselt wird NICHT — bewusst, für eine Einzelnutzer-Maschine.

WOGEGEN DAS SCHÜTZT: die Dateien lagen mitten im Arbeitsbaum. Vor git waren sie sicher
(`.gitignore:36`), vor allem anderen nicht — jedes Sync- und Sicherungswerkzeug, das auf das
Projekt zeigt, nimmt sie mit; beim Kopieren des Ordners wandern sie mit; ein `rm -rf` im
Projektverzeichnis trifft sie. Darin stehen Steuer-ID, Einkommen und IBAN.

WOGEGEN NICHT: die gestohlene Platte. Die Dateien liegen weiterhin im Klartext (0600). Das
war die Wahl, und sie steht hier, damit sie nicht später für ein Versehen gehalten wird.

WAS BEIM UMZUG WIRKLICH DA WAR: keine Falldatei (der Dev-Bestand war vorher aufgeräumt worden),
aber 29 MB Prüfprotokoll — `audit.jsonl` plus ein gepacktes Archiv, beide mit Modus 0644. Sie
führen user_id, fall_id und Aktion, also wer wann welche Steuererklärung bearbeitet hat. Der
Dateirechte-Fix vom Vortag hatte sie nicht erreicht: er wirkt beim ANLEGEN, und der
chmod-Durchlauf lief auf dem falschen Pfad (`faelle/` statt `produkt/haut/faelle/`) und meldete
deshalb „0 Dateien". Erst der Umzug hat es sichtbar gemacht.

NULL LLM.
"""
from __future__ import annotations

import os
import pathlib
import re
import subprocess
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = pathlib.Path(os.path.dirname(HERE))
sys.path.insert(0, str(ROOT / "produkt" / "haut"))

import api_constants as AC  # noqa: E402


def test_der_store_liegt_nicht_im_projektverzeichnis():
    """Der Kern. Ein Pfad unterhalb des Checkouts wäre der Zustand von vorher."""
    faelle = pathlib.Path(AC.FAELLE).resolve()
    assert ROOT.resolve() not in faelle.parents and faelle != ROOT.resolve(), (
        f"Der Fall-Store liegt wieder im Projektverzeichnis: {faelle}\n"
        f"Dort nimmt ihn jedes Sync- und Sicherungswerkzeug mit, das auf das Projekt zeigt.")


def test_der_store_folgt_der_xdg_konvention():
    """Nicht irgendwo ausserhalb, sondern dort, wo Anwendungsdaten unter Linux hingehören —
    damit Sicherungs- und Aufräum-Werkzeuge des Systems ihn finden können."""
    faelle = str(pathlib.Path(AC.FAELLE).resolve())
    xdg = os.environ.get("XDG_DATA_HOME") or os.path.join(os.path.expanduser("~"),
                                                          ".local", "share")
    erwartet = str((pathlib.Path(xdg) / "taxgraph" / "faelle").resolve())
    assert faelle == erwartet, f"{faelle} != {erwartet}"


def test_eine_eigene_wurzel_wird_beachtet(monkeypatch):
    """`$TAXGRAPH_DATEN` überschreibt beides — ohne diesen Weg müsste man für einen anderen Ort
    den Code ändern, und dann tut es irgendwann jemand fest verdrahtet."""
    monkeypatch.setenv("TAXGRAPH_DATEN", "/tmp/taxgraph-probe")
    assert AC._daten_wurzel() == "/tmp/taxgraph-probe"
    monkeypatch.delenv("TAXGRAPH_DATEN")
    monkeypatch.setenv("XDG_DATA_HOME", "/tmp/xdg-probe")
    assert AC._daten_wurzel() == "/tmp/xdg-probe/taxgraph"


def test_das_pruefprotokoll_liegt_bei_den_falldaten():
    """Es führt user_id, fall_id und Aktion — es gehört zu den Falldaten, nicht ins Repository.
    Und `make backup` erfasst es nur mit, WEIL es dort liegt (der Sicherungsbefehl packt genau
    ein Verzeichnis)."""
    sys.path.insert(0, str(ROOT / "produkt" / "store"))
    import audit
    assert pathlib.Path(audit.AUDIT_DIR).resolve() == pathlib.Path(AC.FAELLE).resolve(), (
        f"Protokoll ({audit.AUDIT_DIR}) und Falldaten ({AC.FAELLE}) liegen auseinander — "
        f"dann sichert `make backup` nur eines von beiden und meldet trotzdem Erfolg.")


def test_makefile_und_code_meinen_denselben_ort():
    """Die Naht, an der es still schiefgeht. Zwei Stellen, die denselben Ort meinen, laufen
    auseinander — und wenn FAELLE_ROOT ins Leere zeigt, packt `make backup` ein leeres
    Verzeichnis und meldet Erfolg. Eine Sicherung, die nichts enthält, merkt man genau einmal.

    Geprüft wird der ECHTE Befehl (`make -n backup`), nicht die Variable: die enthält eine
    Fallunterscheidung über $XDG_DATA_HOME, und was am Ende im tar-Aufruf steht, weiss nur
    make. Ein Trockenlauf zeigt genau das und legt dabei nichts an."""
    r = subprocess.run(["make", "-n", "backup"], cwd=str(ROOT),
                       capture_output=True, text=True, timeout=60)
    if r.returncode != 0:
        pytest.skip(f"make nicht verfügbar: {r.stderr[:200]}")
    # `tar czf … -C <FAELLE_ROOT> faelle …` — der Pfad hinter dem ersten -C ist der Ort.
    treffer = re.search(r"-C\s+(\S+)\s+faelle\b", r.stdout)
    assert treffer, f"kein `-C <pfad> faelle` im Sicherungsbefehl:\n{r.stdout[:400]}"
    aus_make = pathlib.Path(treffer.group(1)).expanduser().resolve()
    aus_code = pathlib.Path(AC.FAELLE).parent.resolve()
    assert aus_make == aus_code, (
        f"`make backup` sichert {aus_make}/faelle, der Code schreibt nach {aus_code}/faelle — "
        f"die Sicherung packt dann ein leeres Verzeichnis und meldet Erfolg.")


def test_der_alte_ort_wird_nicht_mehr_beschrieben(tmp_path, monkeypatch):
    """Gegenprobe zum Umzug: ein neu angelegter Fall darf nicht wieder im Projektverzeichnis
    landen. Ohne diese Prüfung wäre eine zurückgedrehte Konstante unbemerkt."""
    import api as API
    import audit
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    monkeypatch.setenv("TAXGRAPH_NO_AUTH", "1")
    API.fall_anlegen({"fall_id": "ort", "scheibe": "gesamt", "veranlagungszeitraum": 2025})

    alt = pathlib.Path(AC.FAELLE_ALT)
    if alt.exists():
        rest = [p.name for p in alt.iterdir() if p.name != ".gitkeep"]
        assert not rest, (
            f"Im alten Ort {alt} liegen wieder Dateien: {rest[:5]} — entweder ist der Umzug "
            f"zurückgedreht, oder etwas schreibt an api_constants.FAELLE vorbei.")


def test_die_dateirechte_gelten_auch_fuer_bestehende_dateien():
    """Der Fund beim Umzug: der Dateirechte-Fix wirkt beim ANLEGEN. Die 29 MB Protokoll, die
    vorher schon dalagen, hatte er nicht erreicht — sie standen weiter auf 0644, und der
    chmod-Durchlauf davor lief auf dem falschen Pfad und meldete deshalb „0 Dateien".

    Diese Prüfung sieht die echten Dateien an, nicht neu erzeugte. Sie überspringt, wo es
    nichts gibt (frischer Klon, CI) — mit Begründung, statt stillschweigend grün zu sein."""
    wurzel = pathlib.Path(AC.FAELLE)
    if not wurzel.is_dir():
        pytest.skip("kein Datenverzeichnis vorhanden (frischer Klon oder CI)")
    dateien = [p for p in wurzel.rglob("*") if p.is_file()]
    if not dateien:
        pytest.skip("Datenverzeichnis ist leer — nichts zu prüfen")
    offen = [f"{p.name}: {oct(p.stat().st_mode & 0o777)}"
             for p in dateien if p.stat().st_mode & 0o077]
    assert not offen, (
        "Dateien im Datenverzeichnis sind für andere Nutzer lesbar:\n  " + "\n  ".join(offen))
