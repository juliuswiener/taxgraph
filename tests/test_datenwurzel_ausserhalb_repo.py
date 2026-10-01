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

import hashlib
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


# ------------------------------------------------------------------ Fingerabdruck des Bestands
# DER ANLASS (2026-10-01): ein Wegwerf-Skript eines Workers hat 15 Falldateien in die ECHTEN
# Nutzerdaten geschrieben (14:06-14:07). Der Bestand wuchs von 192 auf 207. Gefunden hat das
# niemand durch eine Prüfung, sondern durch einen Test, der zufällig eine ZAHL pinnte
# (test_verpflegung_kuerzung_erreicht_xml.NENNER_STAND) — und der zeigte nur die eine Datei,
# die zufällig Verpflegungsfelder trug. Die anderen 14 waren für jede Zählung unsichtbar.
#
# WARUM EINE ZAHL NICHT REICHT: `len(...) == 207` bleibt wahr, wenn eine Datei dazukommt und
# eine verschwindet. Gemessen am 2026-10-01 ist genau das der Fall — 14 der 15 Fremddateien
# tragen keine Verpflegungsfelder, also bewegt sich NENNER_STAND um genau 1, obwohl 15 Dateien
# falsch sind. Ein Zähler ist kein Wächter über eine Menge.
#
# WAS HIER STEHT, ist deshalb die MENGE (Namen), nicht ihre Länge: ein sha256 über die sortierten
# Dateinamen. Jede angelegte, gelöschte oder umbenannte Falldatei ändert ihn — auch bei gleicher
# Anzahl. Die Dateien selbst werden NICHT gelesen (kein Steuer-ID, kein Einkommen, keine IBAN);
# der Hash läuft über Namen. Die NAMEN stehen NICHT im Repo — auch nicht als Liste in der
# Rot-Meldung. Ein Test, der 192 Fall-Dateinamen eincheckt, hätte das Problem, das er bewacht,
# in die Versionsverwaltung kopiert.
#
# GEPINNT IST DER SAUBERE STAND, NICHT DER HEUTIGE — dieselbe Entscheidung wie bei NENNER_STAND
# in test_verpflegung_kuerzung_erreicht_xml.py, und aus demselben Grund: die 15 Fremddateien vom
# 2026-10-01 gehören nicht in den Bestand. Wer den heutigen Stand pinnt (207), macht die
# Verunreinigung dauerhaft und der Test fällt nach dem Aufräumen WIEDER um. Wer den sauberen
# pinnt (192), hat einen Test, der jetzt rot ist — und der nach dem Aufräumen von selbst grün
# wird, ohne dass jemand eine Zahl nachzieht. Genau das ist der Unterschied zwischen einem
# Wächter und einem Protokoll der Gegenwart.
NENNER_SHA256 = "a517f81107b1fef70c43e87e5e1613dbfba1d7089de242365463c88067b21629"
NENNER_ZAHL = 192


def _bestands_namen() -> list[str]:
    """Die Namen der Falldateien im echten Bestand, sortiert. Kein Inhalt, nur Namen."""
    return sorted(p.name for p in pathlib.Path(AC.FAELLE).glob("*.json"))


def _fingerabdruck(namen) -> str:
    return hashlib.sha256("\n".join(sorted(namen)).encode("utf-8")).hexdigest()


def test_der_bestand_hat_sich_nicht_unbemerkt_geaendert():
    """Der Wächter gegen den Fall vom 2026-10-01: fremde Dateien in den echten Nutzerdaten.

    Kein Test kann verhindern, dass ein FREMDER Prozess hier schreibt — diese Wache läuft im
    pytest-Prozess, und das Wegwerf-Skript von damals lief in einem anderen. Was sie kann, ist
    den Zustand FESTHALTEN: der nächste `make unit` sagt dann, dass sich etwas geändert hat.
    Ohne sie fällt eine Verunreinigung nur auf, wenn sie zufällig eine gepinnte Zahl bewegt —
    am 2026-10-01 waren 14 von 15 Fremddateien für jede Zählung unsichtbar.

    ROT IST DERZEIT DER ERWARTETE ZUSTAND (seit 2026-10-01 14:07, 15 Dateien). Das ist der
    Befund, nicht der Fehler: die gepinnte Zahl ist der saubere Stand von 192 Dateien, und der
    Bestand steht bei 207. Diese Wache geht von selbst auf grün, sobald die Fremddateien
    aufgeräumt sind — sie ist die einzige Stelle, die den Aufräum-Erfolg danach BELEGT, statt
    ihn zu behaupten. NICHT auf 207 nachziehen: das macht die Verunreinigung dauerhaft.

    Rot heißt nicht "Test kaputt", sondern "der Bestand ist ein anderer geworden". Die Meldung
    nennt die ZAHL der Abweichung und den Befehl, der die Namen zeigt — die Namen selbst
    bleiben aus dem Repo heraus.
    """
    namen = _bestands_namen()
    if not namen:
        pytest.skip("Kein Bestand vorhanden (frischer Klon oder CI) — dieser Test prüft dann nichts.")

    ist = _fingerabdruck(namen)
    assert ist == NENNER_SHA256, (
        f"Der Falldatei-Bestand hat sich geändert.\n"
        f"  jetzt:        {len(namen)} Dateien, sha256 {ist}\n"
        f"  festgehalten: {NENNER_ZAHL} Dateien (sauberer Stand 2026-10-01), "
        f"sha256 {NENNER_SHA256}\n"
        f"Die Namen stehen absichtlich nicht im Repo. Welche dazukamen oder wegfielen, zeigt:\n"
        f"  ls -lt --time-style=+%Y-%m-%d\\ %H:%M {AC.FAELLE}/*.json | head -30\n"
        f"(die frisch geänderten Dateien stehen oben — am 2026-10-01 waren es 15 zwischen "
        f"14:06 und 14:07, aus einem Wegwerf-Skript, das an api.speichere_fall vorbei schrieb)\n"
        f"War es ein echter Fall: NENNER_ZAHL und NENNER_SHA256 nachziehen. War es Fremdschrift: "
        f"NICHT nachziehen — und die Dateien nicht löschen, dem Verursacher nachgehen.")


def test_fingerabdruck_faellt_wenn_eine_fremde_datei_dazukommt():
    """Negativprobe — ohne sie ist nicht belegt, dass der Fingerabdruck überhaupt anschlägt.

    DER FALL VOM 2026-10-01, nachgestellt: ein fremdes Skript legt eine Falldatei dazu, die
    keine gepinnte ZAHL bewegt. Genau daran war NENNER_STAND blind — 14 der 15 Fremddateien
    trugen keine Verpflegungsfelder und bewegten dort nichts.

    Geprüft wird der harte Fall: EINE Datei dazu, EINE weg — die Länge bleibt gleich, jede
    Zählung ist zufrieden, die Menge ist eine andere. Dazu die Gegenrichtung (unveränderte
    Menge, andere Reihenfolge muss gleich bleiben), damit eine Rot-Meldung nicht aus einem
    Grund kommt, der nichts mit dem Bestand zu tun hat.
    """
    festgehalten = ["a.json", "b.json", "c.json"]

    getauscht = ["a.json", "b.json", "fremd.json"]      # eine dazu, eine weg
    assert len(getauscht) == len(festgehalten), "Tauschprobe muss die gleiche Länge haben"
    assert _fingerabdruck(getauscht) != _fingerabdruck(festgehalten), (
        "Der Fingerabdruck ist blind gegen 'eine Datei dazu, eine weg' — genau der Fall, den "
        "eine reine Zählung nicht sieht.")
    assert _fingerabdruck(festgehalten + ["zan_g.json"]) != _fingerabdruck(festgehalten), (
        "Eine dazugekommene Datei muss den Fingerabdruck ändern.")
    assert _fingerabdruck(["a.json", "b.json"]) != _fingerabdruck(festgehalten), (
        "Eine fehlende Datei muss den Fingerabdruck ändern.")
    assert _fingerabdruck(list(reversed(festgehalten))) == _fingerabdruck(festgehalten), (
        "Die Reihenfolge darf den Fingerabdruck nicht ändern — sonst kommt die Rot-Meldung "
        "auch ohne Bestandsänderung.")
