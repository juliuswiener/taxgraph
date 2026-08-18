"""Kein Unterprozess ohne Zeitlimit — und kein halber Beleg als Ergebnis.

DER FUND (Audit 2026-08-16, res-ocr-subprocess-no-timeout), am 2026-08-18 nachgemessen: neun
`subprocess.run`-Aufrufe auf hochgeladene PDFs, keiner mit `timeout=`. Das wäre in einem
nebenläufigen Server eine hängende Anfrage. Hier ist es mehr:

    server.py:make_server nutzt HTTPServer, NICHT ThreadingHTTPServer — bewusst, weil
    catala_runtime nicht threadsicher ist.

Der Dienst bearbeitet also genau eine Anfrage zur Zeit. Ein `tesseract`, das auf einem
präparierten PDF nicht zurückkehrt, hält damit nicht diese eine Anfrage auf, sondern alles.
Der Auslöser ist ein Upload — der billigste denkbare Angriff auf diesen Server.

WARUM DAS ZEITLIMIT ALLEIN NICHT REICHT: es begrenzt jeden EINZELNEN Aufruf, nicht ihre
Anzahl. Der Teil-Textlayer-Pfad ruft tesseract je Seite auf; 500 Seiten × 60 s sind acht
Stunden Stillstand, wobei jeder einzelne Aufruf brav unter seinem Limit bleibt. Deshalb
zusätzlich ein Deckel auf die Zahl der OCR-pflichtigen Seiten.

WARUM DER DECKEL WERFEN MUSS UND NICHT KÜRZEN: gäbe die Funktion die ersten 40 Seiten zurück,
fehlten die Beträge der übrigen — lautlos, an einer Stelle, die vom Aufrufer nicht von einem
vollständig gelesenen Auszug zu unterscheiden ist. Ein halber Kontoauszug ist gefährlicher als
gar keiner, weil er wie ein Ergebnis aussieht (Klasse: slot-fail-open-get-default, wo eine
falsche Zeile 13.568 EUR löschte und der Zustand "bestaetigt" blieb). Genau das prüft
test_deckel_liefert_kein_teilergebnis — ohne ihn wäre die bequeme Fassung die stille.

NULL LLM, kein tesseract nötig: der Struktur-Teil liest den Quelltext, der Verhaltens-Teil
arbeitet mit einem vorgetäuschten Unterprozess.
"""
from __future__ import annotations

import ast
import os
import pathlib
import subprocess
import sys
import tempfile

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/import", "produkt/traverser",
             "produkt/unsicherheit", "produkt/mapping", "produkt/konsistenz", "produkt/bescheid",
             "golden", "elster"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import kontoauszug_writer as KW   # noqa: E402
import beleg_writer as BW         # noqa: E402

PRODUKT = pathlib.Path(ROOT) / "produkt"

# Unterprozess-Starter, die ein `timeout=` kennen. `Popen` steht bewusst NICHT hier: es nimmt
# gar kein timeout-Argument, sondern verlagert die Wartezeit in .wait()/.communicate() — wer es
# einführt, soll an diesem Gate anhalten und die Wartestelle einzeln begründen, statt sie
# stillschweigend mitzubringen.
STARTER = {"run", "call", "check_call", "check_output"}

# Aufrufe ohne Zeitlimit, die es bleiben dürfen. Jeder Eintrag trägt seinen Grund; die Liste
# darf nur schrumpfen (Muster: AUSNAHMEN in test_zweig_duplikation_differential.py). Heute leer
# — und das ist der ehrlichste Zustand, den sie haben kann.
AUSNAHMEN: dict[str, str] = {}


def _aufrufe_ohne_zeitlimit() -> list[str]:
    """(datei:zeile: befehl) für jeden subprocess-Start unter produkt/ ohne `timeout=`."""
    treffer = []
    for pfad in sorted(PRODUKT.rglob("*.py")):
        if "__pycache__" in str(pfad):
            continue
        baum = ast.parse(pfad.read_text(encoding="utf-8"), filename=str(pfad))
        for n in ast.walk(baum):
            if not (isinstance(n, ast.Call) and isinstance(n.func, ast.Attribute)
                    and n.func.attr in STARTER
                    and isinstance(n.func.value, ast.Name) and n.func.value.id == "subprocess"):
                continue
            if any(kw.arg == "timeout" for kw in n.keywords):
                continue
            rel = pfad.relative_to(pathlib.Path(ROOT))
            # Erstes Listenelement = das Programm, macht die Meldung lesbar
            prog = "?"
            if n.args and isinstance(n.args[0], (ast.List, ast.Tuple)) and n.args[0].elts:
                erst = n.args[0].elts[0]
                if isinstance(erst, ast.Constant):
                    prog = str(erst.value)
            treffer.append(f"{rel}:{n.lineno}: subprocess.{n.func.attr}({prog}...)")
    return treffer


def test_kein_unterprozess_ohne_zeitlimit():
    """Der Kern. Ein Unterprozess ohne Grenze ist in einem einfädigen Server kein
    Bequemlichkeitsproblem, sondern ein Ausschalter, den jeder Upload betätigen kann."""
    offen = [t for t in _aufrufe_ohne_zeitlimit() if t not in AUSNAHMEN]
    assert not offen, (
        "subprocess-Start ohne `timeout=` unter produkt/ — der Server ist einfädig, ein "
        "hängender Unterprozess hält ihn ganz an:\n  " + "\n  ".join(offen))


def test_ausnahmen_sind_begruendet_und_leben_noch():
    """Kein stilles Ausklammern, und keine Liste, die Prüfung vortäuscht: ein Eintrag, den es
    nicht mehr gibt, muss raus."""
    gefunden = set(_aufrufe_ohne_zeitlimit())
    for eintrag, grund in AUSNAHMEN.items():
        assert grund and len(grund) > 20, f"{eintrag}: Ausnahme ohne ausreichende Begründung"
        assert eintrag in gefunden, f"{eintrag} steht in AUSNAHMEN, existiert aber nicht mehr"


def test_das_gate_erkennt_seinen_eigenen_fehlerfall():
    """Negativprobe: ohne sie wäre nicht belegt, dass der Scan überhaupt greift. Ein Gate, das
    seinen Fehlerfall nicht kennt, ist eine Behauptung — und dieses hier ist gerade grün, weil
    alle Aufrufe repariert wurden, also sieht man ihm sein Anschlagen sonst nie an."""
    quelle = ("import subprocess\n"
              "subprocess.run(['tesseract', p], capture_output=True)\n"
              "subprocess.run(['pdftotext', p], timeout=30)\n")
    baum = ast.parse(quelle)
    ohne = [n for n in ast.walk(baum)
            if isinstance(n, ast.Call) and isinstance(n.func, ast.Attribute)
            and n.func.attr in STARTER
            and not any(kw.arg == "timeout" for kw in n.keywords)]
    assert len(ohne) == 1, "das Muster erkennt den ungeschützten Aufruf nicht (oder zu viele)"


# ------------------------------------------------------------- Verhalten: Deckel statt Teilergebnis

def _pdf_mit_seiten(n: int) -> str:
    """pdftotext-Ausgabe für n Seiten, jede ohne brauchbaren Textlayer (< 20 Zeichen, s.
    _textlayer_ist_plausibel) — der Fall, der jede Seite einzeln in die Bilderkennung schickt."""
    return ("x\x0c" * n)


def test_deckel_wirft_statt_zu_kuerzen(monkeypatch, tmp_path):
    """Der eigentliche Schutz. Ein Auszug jenseits des Deckels muss ABBRECHEN, nicht ein
    gekürztes Ergebnis liefern."""
    zu_viel = KW.OCR_SEITEN_HOECHSTZAHL + 1

    def _fake_run(cmd, *a, **kw):
        assert "timeout" in kw, f"Aufruf ohne Zeitlimit durchgerutscht: {cmd}"
        return subprocess.CompletedProcess(cmd, 0, stdout=_pdf_mit_seiten(zu_viel), stderr="")

    monkeypatch.setattr(KW.subprocess, "run", _fake_run)
    pdf = tmp_path / "auszug.pdf"
    pdf.write_bytes(b"%PDF-1.4\n")

    with pytest.raises(KW.OcrZuAufwendig) as e:
        KW.lies_kontoauszug_pdf(str(pdf))
    assert str(zu_viel) in str(e.value), "die Meldung nennt die Seitenzahl nicht"


def test_deckel_liefert_kein_teilergebnis(monkeypatch, tmp_path):
    """Die Gegenprobe zur bequemen Fassung: hätte jemand statt der Ausnahme ein `seiten[:40]`
    geschrieben, wäre der Test oben rot — dieser hier benennt, WARUM das die schlechtere Lösung
    ist. Ein zurückgegebener Text wäre für den Aufrufer nicht von einem vollständigen zu
    unterscheiden, und die fehlenden Beträge fielen niemandem auf."""
    zu_viel = KW.OCR_SEITEN_HOECHSTZAHL + 5
    monkeypatch.setattr(KW.subprocess, "run", lambda cmd, *a, **kw:
                        subprocess.CompletedProcess(cmd, 0, stdout=_pdf_mit_seiten(zu_viel), stderr=""))
    pdf = tmp_path / "auszug.pdf"
    pdf.write_bytes(b"%PDF-1.4\n")
    try:
        text, conf = KW.lies_kontoauszug_pdf(str(pdf))
    except KW.OcrZuAufwendig:
        return                       # richtig: kein Ergebnis
    pytest.fail(
        f"lies_kontoauszug_pdf hat bei {zu_viel} OCR-Seiten ein Ergebnis geliefert "
        f"({len(text)} Zeichen, {len(conf)} Confidence-Einträge) statt abzubrechen — ein "
        f"gekürzter Auszug ist vom vollständigen nicht zu unterscheiden.")


def test_seiten_unter_dem_deckel_laufen_durch(monkeypatch, tmp_path):
    """Der Normalfall muss weiter funktionieren — ohne diesen Test wäre ein Deckel von 0 die
    grünste Lösung (dieselbe Klasse wie ein Gate, das seine eigene Voraussetzung mitbringt)."""
    seiten = 3
    assert seiten <= KW.OCR_SEITEN_HOECHSTZAHL

    def _fake_run(cmd, *a, **kw):
        if cmd[0] == "pdftotext":
            # plausible Textlayer (>= 20 Zeichen je Seite) -> gar kein OCR nötig
            return subprocess.CompletedProcess(cmd, 0, stdout=("Buchung 12,34 EUR am 01.01.2025\x0c" * seiten),
                                               stderr="")
        raise AssertionError(f"unerwarteter Unterprozess im Textlayer-Pfad: {cmd}")

    monkeypatch.setattr(KW.subprocess, "run", _fake_run)
    pdf = tmp_path / "auszug.pdf"
    pdf.write_bytes(b"%PDF-1.4\n")
    text, _conf = KW.lies_kontoauszug_pdf(str(pdf))
    assert "Buchung" in text


# ------------------------------------------------------------------ Naht: kommt es am Endpunkt an?

def _fall_anlegen(tmp_path, monkeypatch) -> str:
    """Minimaler echter Fall über den echten Endpunkt — kein handgebautes Store-Dict, damit der
    Test denselben Weg nimmt wie ein Nutzer."""
    import api as API
    import api_auth
    import audit
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    monkeypatch.setattr(api_auth, "_AUTH_USER", None)
    monkeypatch.setenv("TAXGRAPH_NO_AUTH", "1")
    status, _ = API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025,
                                  "fall_id": "ocr_probe"})
    assert status == 201
    return "ocr_probe"


def _boom_fuer(fehler: str):
    """Der Unterprozess-Ersatz für den jeweiligen Abbruchgrund. Beide werden geprüft, weil sie
    über VERSCHIEDENE Ausnahmetypen laufen (subprocess.TimeoutExpired vs. OcrZuAufwendig) und
    ein `except` leicht nur einen von beiden fängt."""
    if fehler == "timeout":
        def _boom(cmd, *a, **kw):
            raise subprocess.TimeoutExpired(cmd, kw.get("timeout", 30))
    else:
        def _boom(cmd, *a, **kw):
            return subprocess.CompletedProcess(
                cmd, 0, stdout=_pdf_mit_seiten(KW.OCR_SEITEN_HOECHSTZAHL + 1), stderr="")
    return _boom


@pytest.mark.parametrize("fehler", ["timeout", "deckel"])
def test_endpunkt_wirft_apierror_422(tmp_path, monkeypatch, fehler):
    """Die Naht, nicht nur der Writer. Ohne diesen Test wäre belegt, dass der Writer abbricht —
    und offen, was daraus wird: server.py macht aus jeder NICHT als ApiError erkannten Ausnahme
    ein 500 mit nacktem Klassennamen ("TimeoutExpired: Command ... timed out").

    Der Endpunkt gibt bei Erfolg (status, body) zurück und WIRFT im Fehlerfall — die Umwandlung
    in eine HTTP-Antwort macht erst server.py:205. Deshalb wird hier die Ausnahme geprüft und
    der HTTP-Weg im Test darunter."""
    import base64
    import api as API
    fall_id = _fall_anlegen(tmp_path, monkeypatch)
    monkeypatch.setattr(KW.subprocess, "run", _boom_fuer(fehler))

    with pytest.raises(API.ApiError) as e:
        API.kontoauszug(fall_id, {"format": "pdf",
                                  "inhalt": base64.b64encode(b"%PDF-1.4\n").decode()})
    assert e.value.status == 422, (
        f"Endpunkt wirft Status {e.value.status} statt 422 bei '{fehler}'")
    assert "nicht lesbar" in str(e.value), f"Meldung erklärt nichts: {e.value}"


def test_ueber_http_kommt_wirklich_422_an(tmp_path, monkeypatch):
    """Der Beweis, dass es verdrahtet ist. Die ApiError oben nützt nichts, wenn server.py sie
    nicht als solche behandelt — dann sieht der Nutzer ein 500 mit einem Python-Klassennamen
    darin. Dass eine Prüfung den echten Weg nie anfasst, ist hier schon vorgekommen: das
    Beleg-Gate war nie als VERDRAHTET geprüft, und das Login-Backend war monatelang fertig,
    während die Oberfläche nie ein Token schickte."""
    import base64
    import json as _json
    import threading
    import urllib.error
    import urllib.request
    import api as API
    import api_auth
    import audit
    import server as SRV

    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    monkeypatch.setattr(api_auth, "_AUTH_USER", None)
    monkeypatch.setenv("TAXGRAPH_NO_AUTH", "1")
    API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "http_probe"})
    monkeypatch.setattr(KW.subprocess, "run", _boom_fuer("timeout"))

    srv = SRV.make_server(0)
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    basis = f"http://{srv.server_address[0]}:{srv.server_address[1]}"
    try:
        rumpf = _json.dumps({"format": "pdf",
                             "inhalt": base64.b64encode(b"%PDF-1.4\n").decode()}).encode()
        anfrage = urllib.request.Request(
            f"{basis}/fall/http_probe/kontoauszug", data=rumpf, method="POST",
            headers={"Content-Type": "application/json", "Origin": basis})
        try:
            urllib.request.urlopen(anfrage, timeout=15)
            pytest.fail("Endpunkt hat 2xx geantwortet, obwohl der Unterprozess ins Zeitlimit lief")
        except urllib.error.HTTPError as e:
            koerper = e.read().decode()
            assert e.code == 422, (
                f"HTTP {e.code} statt 422 — die Zeitüberschreitung schlägt bis in die "
                f"Allgemein-Behandlung durch:\n{koerper}")
            assert "TimeoutExpired" not in koerper or "nicht lesbar" in koerper, (
                f"nackter Python-Klassenname in der Antwort an den Nutzer:\n{koerper}")
    finally:
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()


def test_endpunkt_laesst_keine_temporaere_pdf_zurueck(tmp_path, monkeypatch):
    """Der Abbruchpfad darf das entpackte PDF nicht liegen lassen: die temporäre Datei trägt den
    ROHEN Auszug samt IBAN, vor jeder Maskierung durch den Writer. Das `finally: os.unlink` gab
    es schon — geprüft war es für diesen neuen Zweig nicht."""
    import base64
    import api as API
    fall_id = _fall_anlegen(tmp_path, monkeypatch)
    vorher = set(pathlib.Path(tempfile.gettempdir()).glob("*.pdf"))

    monkeypatch.setattr(KW.subprocess, "run", _boom_fuer("timeout"))
    with pytest.raises(API.ApiError):
        API.kontoauszug(fall_id, {"format": "pdf",
                                  "inhalt": base64.b64encode(b"%PDF-1.4\n").decode()})

    neu = set(pathlib.Path(tempfile.gettempdir()).glob("*.pdf")) - vorher
    assert not neu, f"temporäre PDF-Datei(en) nach dem Abbruch liegen geblieben: {neu}"


def test_beide_writer_haben_dieselben_grenzen():
    """beleg_writer und kontoauszug_writer halten bewusst je eine eigene Kopie des OCR-Pfads
    (kein Cross-Modul-Import). Zwei Kopien driften — und eine Grenze, die nur in einer der
    beiden nachgezogen wird, ist genau die Bugklasse, die hier schon zweimal Geld gekostet hat
    (kist-bemessungsgrundlage-doppelbug.md)."""
    for name in ("PDFTOTEXT_ZEITLIMIT_S", "PDFTOPPM_ZEITLIMIT_S", "TESSERACT_ZEITLIMIT_S",
                 "OCR_SEITEN_HOECHSTZAHL"):
        assert getattr(KW, name) == getattr(BW, name), (
            f"{name} ist in den beiden Writern verschieden: kontoauszug={getattr(KW, name)}, "
            f"beleg={getattr(BW, name)} — eine der beiden Kopien wurde nachgezogen, die andere nicht.")
