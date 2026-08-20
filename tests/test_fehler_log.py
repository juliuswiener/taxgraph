"""Gate für die Fehlerprotokollierung — beide Hälften.

(a) Ein unerwarteter Fehler darf nicht ungeloggt verschwinden.
(b) Genauso wichtig: es darf keine PII im Protokoll landen.

Ohne (b) besteht `logger.exception(e)` die erste Hälfte glänzend und schreibt dabei
Steuerbeträge auf die Platte. Ohne (a) besteht ein Protokoll, das NICHTS schreibt, die
zweite Hälfte perfekt — deshalb steht test_ursprungsort_ist_brauchbar hier daneben und
prüft, dass der Eintrag diagnostisch etwas taugt.

Der PII-Verdacht ist gemessen, nicht angenommen: produkt/store/store.py:232 und :342 werfen
`ValueError(f"... {feld_id}={wert!r} ...")` — der abgewiesene Betrag steht im Ausnahmetext —
und produkt/haut/server.py reicht ihn als `f"{type(e).__name__}: {e}"` an den Nutzer weiter.
Genau dieser Meldungstext wird unten durch die scharfe Route geschickt.

KEIN Mock-LLM: der Chat-Zweig läuft in seinen ECHTEN Ausfall (conftest entfernt LLM_API_KEY,
llm_client wirft daraufhin von selbst), der ORS-Zweig ebenso über einen gelöschten Schlüssel.
Es wird keine LLM-Antwort gefälscht und kein Netz berührt.
"""
from __future__ import annotations

import ast
import json
import os
import pathlib
import sys
import threading
import urllib.error
import urllib.request

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import api as API          # noqa: E402
import audit               # noqa: E402
import fehler_log as FL    # noqa: E402
import server as SRV       # noqa: E402

# Echte PII-Formen, wie sie in diesem Produkt vorkommen. Neunstellig bzw. wörtlich, damit ein
# Treffer kein Zufall aus einem Zeitstempel sein kann.
IBAN = "DE89370400440532013000"
STEUER_ID = "12345678901"
BETRAG = "918273645"                       # Cent-Wert, wie ihn store.py:342 in {wert!r} setzt
NAME = "Erika Musterfrau"
ART9 = "Schwerbehinderung GdB 80"          # Gesundheitsdatum, Art. 9 DSGVO
MARKER = (IBAN, STEUER_ID, BETRAG, NAME, ART9)

# Der Meldungstext ist dem echten aus store.py:342 nachgebildet — dort steht {wert!r}.
PII_MELDUNG = (f"fail-closed (F2/Magnitude): kap_ertraege={BETRAG!r} von llm:chat — "
               f"Konto {IBAN}, StNr {STEUER_ID}, {NAME}, {ART9}")


@pytest.fixture
def base(tmp_path, monkeypatch):
    """Server auf einem freien Port; Falldaten UND beide Protokolle in tmp_path.

    `audit.AUDIT_DIR` ist die eine Wegbeschreibung — fehler_log._pfad() liest sie zur
    Aufrufzeit und folgt automatisch. Damit greift zugleich die conftest-Wache nicht,
    weil der aufgelöste Pfad nicht mehr der echte ist.
    """
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    srv = SRV.make_server(0)
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    try:
        yield f"http://{srv.server_address[0]}:{srv.server_address[1]}"
    finally:
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()


def _ruf(base: str, method: str, pfad: str, body: dict | None = None) -> int:
    daten = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + pfad, data=daten, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            return r.status
    except urllib.error.HTTPError as e:
        e.read()
        return e.code


def _roh() -> str:
    """Die Protokolldatei als ROHER Text — nicht als geparstes JSON. Ein Test, der nur die
    bekannten Schlüssel prüft, übersieht PII in einem Feld, an das er nicht gedacht hat."""
    pfad = FL._pfad()
    if not os.path.exists(pfad):
        return ""
    return pathlib.Path(pfad).read_text(encoding="utf-8")


# ------------------------------------------------------------------ (a) nichts verschwindet

def test_unerwarteter_fehler_landet_im_protokoll(base, monkeypatch):
    """server.py fängt jeden unerwarteten Fehler aus jeder Route ab und antwortet 500.
    Ohne Protokoll ist danach weder Typ noch Ort bekannt."""
    def _platzt(_fall_id):
        raise ValueError(PII_MELDUNG)

    monkeypatch.setattr(API, "stand", _platzt)
    assert _ruf(base, "GET", "/fall/abc123/stand") == 500

    eintraege = FL.lies()
    assert eintraege, "der 500er hinterliess KEINEN Protokolleintrag"
    e = eintraege[-1]
    assert e["typ"] == "ValueError"
    assert e["stufe"] == "ERROR"
    assert "server.dispatch" in e["ort"]
    assert e["fall_id"] == "abc123"


def test_ursprungsort_ist_brauchbar(base, monkeypatch):
    """Gegengewicht zur PII-Hälfte: ein Protokoll, das nichts schreibt, wäre trivial
    PII-frei. Der Eintrag muss Datei, Zeile und Funktion nennen — und der Dateipfad muss
    repo-relativ sein, damit nicht der Benutzername des Rechners mitläuft."""
    def _platzt(_fall_id):
        raise RuntimeError("egal")

    monkeypatch.setattr(API, "stand", _platzt)
    _ruf(base, "GET", "/fall/abc123/stand")

    quelle = FL.lies()[-1]["quelle"]
    datei, zeile, funktion = quelle.rsplit(":", 2)
    assert datei.startswith("tests/"), f"Pfad nicht repo-relativ: {quelle}"
    assert not os.path.isabs(datei) and "/home/" not in datei
    assert zeile.isdigit() and int(zeile) > 0
    assert funktion == "_platzt", f"innerster Rahmen falsch: {quelle}"


def test_erwarteter_ausfall_behaelt_seinen_grund(base, monkeypatch):
    """api.entfernung fing Cap-Gate, Netzfehler und Import bisher ohne `as e` — alle drei
    sahen von aussen identisch aus. ECHTER Ausfallpfad: ohne Schlüssel wirft ors_client
    selbst, es wird kein Netz berührt und nichts gefälscht."""
    monkeypatch.delenv("ORS_API_KEY", raising=False)
    _ruf(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025,
                                 "fall_id": "ors1"})
    assert _ruf(base, "POST", "/fall/ors1/entfernung",
                {"von": "A-Str 1, Berlin", "nach": "B-Weg 2, Berlin"}) == 503

    passend = [e for e in FL.lies() if e["ort"] == "api.entfernung ors"]
    assert passend, "der ORS-Ausfall wurde nicht protokolliert"
    assert passend[-1]["stufe"] == "WARNING"        # erwartet -> WARNUNG, nicht FEHLER
    assert passend[-1]["typ"] == "OrsNichtVerfuegbar"


# ------------------------------------------------------------------ (b) keine PII

def test_kein_pii_im_protokoll(base, monkeypatch):
    """Die Kernhälfte. Der Ausnahmetext trägt Betrag, IBAN, Steuer-ID, Name und ein
    Gesundheitsdatum; die Antwort an den Nutzer enthält ihn (das ist bestehendes,
    hier nicht geändertes Verhalten) — die Protokolldatei darf ihn NICHT enthalten."""
    def _platzt(_fall_id):
        raise ValueError(PII_MELDUNG)

    monkeypatch.setattr(API, "stand", _platzt)
    assert _ruf(base, "GET", "/fall/abc123/stand") == 500

    roh = _roh()
    assert roh, "nichts geschrieben — dann prueft dieser Test nichts"
    for m in MARKER:
        assert m not in roh, f"PII im Fehler-Protokoll: {m!r} steht in {FL._pfad()}"
    # Auch kein Teilstück des Meldungstexts: `str(exc)` darf gar nicht erst gelesen werden.
    assert "fail-closed" not in roh and "F2/Magnitude" not in roh


def test_meta_nimmt_keinen_text(tmp_path, monkeypatch):
    """Zusatzangaben sind Anzahlen und Wahrheitswerte. Ein String wird nicht geschrieben,
    sondern durch seinen Typnamen ersetzt — Text ist die Form, in der Nutzdaten reisen,
    und eine Regel, die von der Sorgfalt des naechsten Aufrufers abhaengt, haelt nicht."""
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path))
    FL.protokolliere("test.meta", ValueError(PII_MELDUNG),
                     anzahl=3, geglueckt=False, leer=None, verraeterisch=IBAN)

    roh = _roh()
    assert IBAN not in roh
    e = FL.lies()[-1]
    assert e["anzahl"] == 3 and e["geglueckt"] is False and e["leer"] is None
    assert e["verraeterisch"] == "<str>"


def test_traceback_text_wird_nicht_gelesen(tmp_path, monkeypatch):
    """`traceback.extract_tb` liefert je Rahmen auch `.line`, den QUELLTEXT der Zeile.
    Steht dort ein Literal mit Nutzdaten, waere es damit im Protokoll."""
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path))
    try:
        raise ValueError(f"Kontonummer {IBAN} abgewiesen")   # <- diese Quellzeile traegt PII
    except ValueError as e:
        FL.protokolliere("test.traceback", e)

    roh = _roh()
    assert IBAN not in roh, "der Quelltext der Fehlerzeile ist ins Protokoll gelangt"
    assert "Kontonummer" not in roh
    assert FL.lies()[-1]["typ"] == "ValueError"      # der Eintrag existiert trotzdem


# ------------------------------------------------------------------ Struktur: die Schranke umgehbar?

def _py_dateien() -> list[pathlib.Path]:
    return sorted(pathlib.Path(ROOT, "produkt").rglob("*.py"))


def test_nur_fehler_log_benutzt_logging_direkt():
    """Ohne diese Schranke ist das Gate oben umgehbar: ein `logger.exception(e)` irgendwo
    in produkt/ schreibt `str(e)` und damit den Steuerbetrag, ohne je durch protokolliere()
    zu laufen. Deshalb darf genau EIN Modul `logging` kennen."""
    treffer = []
    for pfad in _py_dateien():
        if pfad.name == "fehler_log.py":
            continue
        baum = ast.parse(pfad.read_text(encoding="utf-8"), filename=str(pfad))
        for n in ast.walk(baum):
            if isinstance(n, ast.Import) and any(a.name.split(".")[0] == "logging"
                                                 for a in n.names):
                treffer.append(f"{pfad.relative_to(ROOT)}:{n.lineno} import logging")
            elif isinstance(n, ast.ImportFrom) and (n.module or "").split(".")[0] == "logging":
                treffer.append(f"{pfad.relative_to(ROOT)}:{n.lineno} from logging import")
    assert not treffer, (
        "logging wird ausserhalb von produkt/store/fehler_log.py benutzt — dort gilt die "
        "PII-Schranke nicht:\n  " + "\n  ".join(treffer) +
        "\nFehler bitte ueber fehler_log.protokolliere(ort, exc) melden.")


def test_kein_aufruf_reicht_einen_ausnahmetext_durch():
    """protokolliere() liest `str(exc)` nicht — aber ein Aufrufer koennte den Text als
    `ort` oder als meta-Angabe selbst hineinreichen. `ort` muss deshalb ein Literal sein
    (ein f-String darf nur aus Konstanten bestehen), und kein Argument darf ein
    str()-Aufruf sein."""
    verstoesse = []
    for pfad in _py_dateien():
        baum = ast.parse(pfad.read_text(encoding="utf-8"), filename=str(pfad))
        for n in ast.walk(baum):
            if not (isinstance(n, ast.Call) and isinstance(n.func, ast.Attribute)
                    and n.func.attr == "protokolliere"):
                continue
            stelle = f"{pfad.relative_to(ROOT)}:{n.lineno}"
            for arg in list(n.args) + [k.value for k in n.keywords]:
                if isinstance(arg, ast.Call) and isinstance(arg.func, ast.Name) \
                        and arg.func.id in ("str", "repr", "format"):
                    verstoesse.append(f"{stelle}: {arg.func.id}(...) als Argument")
            if n.args and isinstance(n.args[0], ast.JoinedStr):
                # f-String als `ort` ist erlaubt, solange jedes eingesetzte Stueck ein
                # Attributzugriff auf ein Muster/Literal ist -- ein blosser Name koennte
                # eine Nutzereingabe sein.
                for teil in n.args[0].values:
                    if isinstance(teil, ast.FormattedValue) and isinstance(teil.value, ast.Name):
                        verstoesse.append(f"{stelle}: freie Variable im ort-f-String")
    assert not verstoesse, (
        "ein Aufruf reicht moeglicherweise Nutzdaten am PII-Schutz vorbei:\n  "
        + "\n  ".join(verstoesse))
