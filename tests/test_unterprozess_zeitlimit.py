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
for _sub in ("produkt/haut", "produkt/store", "produkt/eingang", "produkt/traverser",
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


def test_scan_menge_ist_nicht_leer_und_enthaelt_reale_module():
    """Selbstprüfung des Scans: die Scan-Basis, die _aufrufe_ohne_zeitlimit tatsächlich
    durchläuft (PRODUKT.rglob("*.py")), darf nicht leer sein und muss mindestens die zum
    Zeitpunkt dieses Tests bekannten realen Module unter produkt/ enthalten. Ohne diese
    Zusicherung wäre test_kein_unterprozess_ohne_zeitlimit bei einem stumm kollabierten Scan
    (verschobenes produkt/-Verzeichnis, falscher PRODUKT-Pfad) grün, obwohl er nichts geprüft
    hätte — heute ist die Fundliste ohnehin leer (repariert), also sieht man dem Gate einen
    kollabierten Scan gerade NICHT an (Muster: test_llm_import_boundary.py::
    test_scan_menge_ist_nicht_leer_und_enthaelt_reale_module)."""
    gescannt = {p.relative_to(pathlib.Path(ROOT)) for p in PRODUKT.rglob("*.py")
                if "__pycache__" not in str(p)}
    UNTERGRENZE = 30  # gemessen 2026-09-08: 36 .py-Dateien unter produkt/
    assert len(gescannt) >= UNTERGRENZE, (
        f"Nur {len(gescannt)} .py-Dateien unter produkt/ gefunden, erwartet mindestens "
        f"{UNTERGRENZE} — die Scan-Basis ist unerwartet klein geworden"
    )
    bekannte_module = {pathlib.Path("produkt/haut/api.py"), pathlib.Path("produkt/store/store.py")}
    fehlend = bekannte_module - gescannt
    assert not fehlend, f"Bekannte Module fehlen in der Scan-Basis: {sorted(fehlend)}"


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
    elif fehler == "werkzeug_fehlt":
        # So meldet subprocess.run ein Programm, das auf dem PATH nicht liegt.
        def _boom(cmd, *a, **kw):
            raise FileNotFoundError(2, "No such file or directory", "pdftotext")
    elif fehler == "pdftoppm_fehler":
        _boom = _run_werkzeuge(pdftoppm_rc=1)
    elif fehler == "tesseract_fehler":
        _boom = _run_werkzeuge(tesseract_rc=1)
    elif fehler == "programmfehler":
        # Ein Fehler, der kein Betriebsproblem ist: muss ein 500 bleiben, kein 503.
        def _boom(cmd, *a, **kw):
            raise ValueError("kein Betriebsproblem")
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


TSV_KOPF = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n"
EINE_LEERE_SEITE = "x\x0c"      # pdftotext-Ausgabe: eine Seite, zu kurz fuer einen Textlayer -> Einzelseiten-OCR


def _run_werkzeuge(*, fehlt: str | None = None, text: str = "", pdftoppm_rc: int = 0,
                   tesseract_rc: int = 0):
    """Unterprozess-Ersatz fuer pdftotext, pdftoppm und tesseract. `text` ist die pdftotext-Ausgabe
    (leer = Voll-Scan, EINE_LEERE_SEITE = Einzelseite). `fehlt` nennt ein Programm, das nicht auf dem
    PATH liegt. Bei Exit 0 legt pdftoppm eine Seite ab, bei Exit != 0 keine (so gemessen); tesseract
    liefert bei Exit 0 nur die TSV-Kopfzeile (die weisse Seite), bei Exit != 0 nichts."""
    def _run(cmd, *a, **kw):
        if cmd[0] == fehlt:
            raise FileNotFoundError(2, "No such file or directory", fehlt)
        if cmd[0] == "pdftotext":
            return subprocess.CompletedProcess(cmd, 0, stdout=text, stderr="")
        if cmd[0] == "pdftoppm":
            if pdftoppm_rc == 0:
                pathlib.Path(cmd[-1] + "-1.png").write_bytes(b"")
            return subprocess.CompletedProcess(cmd, pdftoppm_rc, stdout=b"", stderr=b"")
        if cmd[0] == "tesseract":
            return subprocess.CompletedProcess(cmd, tesseract_rc, stderr="",
                                               stdout=TSV_KOPF if tesseract_rc == 0 else "")
        raise AssertionError(f"unerwarteter Unterprozess: {cmd}")
    return _run


@pytest.mark.parametrize("fehlt", ["pdftotext", "pdftoppm", "tesseract"])
def test_endpunkt_fehlendes_werkzeug_wirft_apierror_503(tmp_path, monkeypatch, fehlt):
    """Ein Programm, das auf dem Rechner fehlt, ist ein Betriebsproblem: 503 mit Klartext (Vault
    decisions/fehlendes-hilfsprogramm-antwortet-503). Alle drei Programme laufen über dieselbe
    Stelle; geprüft wird jedes, weil pdftoppm und tesseract erst im OCR-Zweig erreicht werden.
    Der Ausnahme-Typ und die errno-Zeile gehören nicht in die Meldung an den Nutzer."""
    import base64
    import api as API
    fall_id = _fall_anlegen(tmp_path, monkeypatch)
    monkeypatch.setattr(KW.subprocess, "run", _run_werkzeuge(fehlt=fehlt))

    with pytest.raises(API.ApiError) as e:
        API.kontoauszug(fall_id, {"format": "pdf",
                                  "inhalt": base64.b64encode(b"%PDF-1.4\n").decode()})
    meldung = str(e.value)
    assert e.value.status == 503, f"Status {e.value.status} statt 503: {meldung}"
    assert "FileNotFoundError" not in meldung and "Errno" not in meldung, (
        f"Ausnahme-Typ in der Meldung an den Nutzer: {meldung}")
    # Wortgleich mit Rust (rust/api/tests/kontoauszug_werkzeug.rs): zwei Texte, die auseinanderlaufen,
    # sieht kein Test, solange jeder nur sein eigenes Stück prüft.
    assert meldung == (f"PDF-Auslesen ist gerade nicht möglich: Das Programm '{fehlt}' fehlt auf "
                       f"diesem Rechner."), f"Wortlaut weicht ab: {meldung}"


def test_endpunkt_anderer_fehler_bleibt_kein_apierror(tmp_path, monkeypatch):
    """Gegenprobe zum 503: nur FileNotFoundError ist ein Betriebsproblem. Ein ValueError aus dem
    Lesepfad darf NICHT zu einem ApiError werden (kein Catch-all), sonst versteckt der Zweig
    echte Programmfehler. ApiError ist selbst eine ValueError-Unterklasse, daher der isinstance."""
    import base64
    import api as API
    fall_id = _fall_anlegen(tmp_path, monkeypatch)
    monkeypatch.setattr(KW.subprocess, "run", _boom_fuer("programmfehler"))

    with pytest.raises(ValueError) as e:
        API.kontoauszug(fall_id, {"format": "pdf",
                                  "inhalt": base64.b64encode(b"%PDF-1.4\n").decode()})
    assert not isinstance(e.value, API.ApiError), f"ValueError wurde zum ApiError: {e.value}"


@pytest.mark.parametrize("weg", ["voll", "einzel"])
@pytest.mark.parametrize("programm, status", [("pdftoppm", 422), ("tesseract", 503)])
def test_endpunkt_rueckgabecode_pdftoppm_422_tesseract_503(tmp_path, monkeypatch, programm, status,
                                                            weg):
    """Endet pdftoppm oder tesseract mit Fehlercode, scheitert der Upload (Vault decisions/ein-
    hilfsprogramm-mit-fehlercode-bricht-den-upload-ab) — vorher las er still leer (Voll-Scan) oder
    brach mit IndexError ab (Einzelseite). pdftoppm -> 422: die Datei laesst sich nicht in Bilder
    umwandeln, der Nutzer kann eine andere hochladen. tesseract -> 503: das Bild hat pdftoppm gerade
    selbst gemacht, der einzige gemessene Ausloeser ist eine unvollstaendige Installation (fehlende
    deu-Sprachdaten). Beide Wege (Voll-Scan, Einzelseite) und beide Programme."""
    import base64
    import api as API
    fall_id = _fall_anlegen(tmp_path, monkeypatch)
    text = "" if weg == "voll" else EINE_LEERE_SEITE
    monkeypatch.setattr(KW.subprocess, "run",
                        _run_werkzeuge(text=text, **{f"{programm}_rc": 1}))

    with pytest.raises(API.ApiError) as e:
        API.kontoauszug(fall_id, {"format": "pdf",
                                  "inhalt": base64.b64encode(b"%PDF-1.4\n").decode()})
    meldung = str(e.value)
    assert e.value.status == status, f"Status {e.value.status} statt {status}: {meldung}"
    for name in ("PdfNichtLesbar", "OcrNichtVerfuegbar", "IndexError"):
        assert name not in meldung, f"Ausnahme-Typ {name} in der Meldung: {meldung}"
    if status == 422:
        assert "nicht lesbar" in meldung and "Bilder" in meldung, f"Meldung erklärt nichts: {meldung}"
    else:
        assert meldung.startswith("PDF-Auslesen ist gerade nicht möglich:"), meldung


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


def _kontoauszug_ueber_http(tmp_path, monkeypatch, run) -> tuple[int, str]:
    """Schickt einen PDF-Kontoauszug über den echten Server (server.py) und gibt (Status, Körper)
    zurück, auch bei 4xx/5xx. `run` ersetzt subprocess.run. Gleicher Weg wie
    test_ueber_http_kommt_wirklich_422_an, nur ohne feste Erwartung."""
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
    monkeypatch.setattr(KW.subprocess, "run", run)

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
            with urllib.request.urlopen(anfrage, timeout=15) as antwort:
                return antwort.status, antwort.read().decode()
        except urllib.error.HTTPError as e:
            return e.code, e.read().decode()
    finally:
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()


def test_ueber_http_kommt_wirklich_503_an(tmp_path, monkeypatch):
    """Fehlt ein Lesewerkzeug, sieht der Nutzer über den echten Server ein 503 mit Klartext —
    kein 500 mit "FileNotFoundError: [Errno 2] ..." (roter Befehl A22 der Entscheidung
    fehlendes-hilfsprogramm-antwortet-503)."""
    code, koerper = _kontoauszug_ueber_http(
        tmp_path, monkeypatch, _boom_fuer("werkzeug_fehlt"))
    assert code == 503, f"HTTP {code} statt 503:\n{koerper}"
    assert "FileNotFoundError" not in koerper and "Errno" not in koerper, (
        f"Ausnahme-Typ in der Antwort an den Nutzer:\n{koerper}")
    assert "pdftotext" in koerper, f"Antwort nennt das fehlende Programm nicht:\n{koerper}"


def test_ueber_http_bleibt_ein_anderer_fehler_500(tmp_path, monkeypatch):
    """Gegenprobe über den echten Server: ein ValueError aus dem Lesepfad bleibt ein 500."""
    code, koerper = _kontoauszug_ueber_http(
        tmp_path, monkeypatch, _boom_fuer("programmfehler"))
    assert code == 500, f"HTTP {code} statt 500:\n{koerper}"


@pytest.mark.parametrize("fehler, status, text", [
    ("pdftoppm_fehler", 422, "nicht lesbar"),
    ("tesseract_fehler", 503, "PDF-Auslesen ist gerade nicht möglich"),
])
def test_ueber_http_rueckgabecode_pdftoppm_422_tesseract_503(tmp_path, monkeypatch, fehler, status,
                                                              text):
    """Der Beweis ueber server.py: ein Fehlercode von pdftoppm kommt als 422, einer von tesseract als
    503 beim Nutzer an — kein 200 mit 0 Buchungen, kein 500 mit Klassenname. Der Zweig fuer tesseract
    sitzt hinter dem fuer FileNotFoundError und liefert dieselbe Art Antwort."""
    code, koerper = _kontoauszug_ueber_http(
        tmp_path, monkeypatch, _boom_fuer(fehler))
    assert code == status, f"HTTP {code} statt {status}:\n{koerper}"
    assert text in koerper, f"Antwort enthält {text!r} nicht:\n{koerper}"
    for name in ("PdfNichtLesbar", "OcrNichtVerfuegbar", "IndexError", "FileNotFoundError"):
        assert name not in koerper, f"Ausnahme-Typ {name} in der Antwort an den Nutzer:\n{koerper}"


@pytest.mark.parametrize("fehler", ["timeout", "werkzeug_fehlt", "pdftoppm_fehler", "tesseract_fehler"])
def test_endpunkt_laesst_keine_temporaere_pdf_zurueck(tmp_path, monkeypatch, fehler):
    """Der Abbruchpfad darf das entpackte PDF nicht liegen lassen: die temporäre Datei trägt den
    ROHEN Auszug samt IBAN, vor jeder Maskierung durch den Writer. Das `finally: os.unlink` gab
    es schon — geprüft war es für diesen neuen Zweig nicht. Gilt für jeden Abbruchgrund, der als
    eigener `except`-Zweig dazukommt (zuletzt: ein fehlendes Hilfsprogramm)."""
    import base64
    import api as API
    fall_id = _fall_anlegen(tmp_path, monkeypatch)
    vorher = set(pathlib.Path(tempfile.gettempdir()).glob("*.pdf"))

    monkeypatch.setattr(KW.subprocess, "run", _boom_fuer(fehler))
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


# ------------------------------------------------------ Verhalten: tesseract rechnet mit einem Faden

# Die vier tesseract-Aufrufe, je über die öffentliche Einstiegsfunktion erreicht. pdftotext
# liefert entweder gar keinen Text (Voll-Scan) oder eine plausible Seite 1 und eine leere
# Seite 2 (Teil-Textlayer, nur Seite 2 geht in die Bilderkennung).
TESSERACT_WEGE = {
    "kontoauszug_voll_scan": (KW, KW.lies_kontoauszug_pdf, ""),
    "kontoauszug_teil_scan": (KW, KW.lies_kontoauszug_pdf, "Buchung 12,34 EUR am 01.01.2025\x0c\x0c"),
    "beleg_voll_scan": (BW, BW.lies_beleg_text, ""),
    "beleg_teil_scan": (BW, BW.lies_beleg_text, "Lohnsteuerbescheinigung 2025 Nr. 3\x0c\x0c"),
}


@pytest.mark.parametrize("weg", sorted(TESSERACT_WEGE))
def test_tesseract_rechnet_mit_einem_faden(monkeypatch, tmp_path, weg):
    """tesseract startet sonst mehrere OpenMP-Fäden, und unter Überbuchung brach ein Scan, der
    allein 0,25 s braucht, am Zeitlimit ab (2026-09-26, Last 27,8 auf 12 Kernen). Das Limit muss
    AUSDRÜCKLICH am Aufruf stehen, und die übrige Umgebung muss erhalten bleiben — ohne PATH
    fände der Unterprozess tesseract nicht mehr."""
    modul, lies, pdftotext_ausgabe = TESSERACT_WEGE[weg]
    monkeypatch.setenv("TG_UMGEBUNG_ZUR_LAUFZEIT", "ja")   # nach dem Import gesetzt
    envs = []

    def _fake_run(cmd, *a, **kw):
        if cmd[0] == "pdftotext":
            return subprocess.CompletedProcess(cmd, 0, stdout=pdftotext_ausgabe, stderr="")
        if cmd[0] == "pdftoppm":
            open(cmd[-1] + "-1.png", "wb").close()   # der Writer liest die PNG aus dem Verzeichnis
            return subprocess.CompletedProcess(cmd, 0, stdout=b"", stderr=b"")
        assert cmd[0] == "tesseract", f"unerwarteter Unterprozess: {cmd}"
        envs.append(kw.get("env"))
        return subprocess.CompletedProcess(cmd, 0, stdout="", stderr="")

    monkeypatch.setattr(modul.subprocess, "run", _fake_run)
    pdf = tmp_path / "scan.pdf"
    pdf.write_bytes(b"%PDF-1.4\n")
    lies(str(pdf))

    n = len(envs)
    assert n == 1, f"{weg}: {n} tesseract-Aufrufe erreicht statt genau einem"
    env = envs[0]
    assert env is not None, f"{weg}: tesseract ohne env= aufgerufen, OMP_THREAD_LIMIT fehlt"
    soll = {**os.environ, "OMP_THREAD_LIMIT": "1"}
    # Nur Namen in die Meldung, nie Werte: die Umgebung trägt API-Schlüssel.
    abweichend = sorted(k for k in soll.keys() | env.keys() if env.get(k) != soll.get(k))
    assert not abweichend, f"{weg}: env= weicht von der Umgebung ab bei {abweichend}"


# ------------------------------------------------- Verhalten: der pdftotext-Exit entscheidet, nicht der Text

@pytest.mark.parametrize("exit_code", [1, 2, 3, 99, -9])
@pytest.mark.parametrize("weg", ["kontoauszug_voll_scan", "beleg_voll_scan"])
def test_pdftotext_exit_code(monkeypatch, tmp_path, weg, exit_code):
    """Exit 3 (Rechte-Fehler) liefert keinen Text, die Seiten lassen sich aber rastern: weiter in die
    Bilderkennung wie bei einem Scan. Jeder andere Code≠0 heißt „nicht lesbar" und ist ein Fehler —
    vorher wurde daraus lautlos ein leerer Auszug. Vorgetäuscht, weil der poppler-Build dieser
    Maschine die Rechte gar nicht prüft: ein kopiergeschütztes PDF endet dort mit Exit 0."""
    modul, lies, _ = TESSERACT_WEGE[weg]
    gerufen = []

    def _fake_run(cmd, *a, **kw):
        gerufen.append(cmd[0])
        if cmd[0] == "pdftotext":
            return subprocess.CompletedProcess(cmd, exit_code, stdout="", stderr="")
        if cmd[0] == "pdftoppm":
            open(cmd[-1] + "-1.png", "wb").close()
            return subprocess.CompletedProcess(cmd, 0, stdout=b"", stderr=b"")
        return subprocess.CompletedProcess(cmd, 0, stdout="", stderr="")

    monkeypatch.setattr(modul.subprocess, "run", _fake_run)
    pdf = tmp_path / "x.pdf"
    pdf.write_bytes(b"%PDF-1.4\n")
    if exit_code == 3:
        lies(str(pdf))
        assert "tesseract" in gerufen, f"{weg}: Exit 3 erreicht die Bilderkennung nicht: {gerufen}"
    else:
        with pytest.raises(modul.PdfNichtLesbar):
            lies(str(pdf))
        assert gerufen == ["pdftotext"], f"{weg}: nach Exit {exit_code} lief noch {gerufen[1:]}"
