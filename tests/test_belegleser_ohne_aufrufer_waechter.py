"""Wächter: `lies_beleg_text` hat unter produkt/ keinen Aufrufer — und bekommt keinen, bevor er repariert ist.

Warum: ein Beleg als reiner Scan (PDF ohne Textebene) wird von `lies_beleg_text` als LEERER Text gelesen.
`beleg_writer.lies_beleg_text` gibt das ganze PDF an `tesseract`, das kein PDF liest (Exit 1), und der
Exit wird nicht geprüft. Heute erreicht das keinen Nutzer, weil kein Produktcode die Funktion aufruft
(nur Tests und das Paritäts-Orakel `tools/parity/schritt8_oracle.py`). Sobald ein Beleg-Upload sie
aufruft, sieht ein gescannter Beleg aus wie ein Beleg ohne Beträge: der Nutzer verliert Abzüge, ohne es
zu merken.

Die Zusage "wer die Funktion zum ersten Mal an einen Nutzerpfad hängt, baut die Umwandlung" stand schon
einmal ohne Durchsetzung da. Dieser Test setzt sie durch: er zählt per `ast` jede Verwendung von
`lies_beleg_text` unter produkt/ und schlägt fehl, sobald eine entsteht. Die Meldung nennt das Ticket und
das Rezept.

Ticket:      backlog/taxgraph/beleg-pdf-ohne-textlayer-wird-leer-gelesen.md (AK1)
Entscheidung: decisions/belegleser-bleibt-unrepariert-bis-der-waechter-meldet.md

Gezählt wird per `ast`, nicht per Grep: das Wort steht in produkt/ dreimal in Kommentaren und Docstrings
(`api.py`, `kontoauszug_writer.py`, `beleg_writer.py`), und diese Treffer sind keine Aufrufer.
Eine Verwendung ist ein Name `lies_beleg_text`, ein Attribut `….lies_beleg_text` (Aufruf wie
`BW.lies_beleg_text(p)` UND weitergereicht wie `pool.submit(BW.lies_beleg_text, p)`) oder ein
`from … import lies_beleg_text`. Die Definition zählt nicht.

ponytail: Obergrenze. `from beleg_writer import lies_beleg_text as lb` meldet der Wächter schon am Import.
`getattr(BW, "lies_beleg_text")` und ein dynamischer `importlib`-Import meldet er nicht: wer so verdrahtet,
umgeht ihn. Das ist Absicht, kein Versehen, und der Wächter soll das Versehen fangen. Der Wächter gilt nur
für Python und nur unter produkt/; ein Rust-Gegenstück gibt es nicht.
"""
from __future__ import annotations

import ast
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PRODUKT = os.path.join(ROOT, "produkt")

NAME = "lies_beleg_text"
TICKET = "beleg-pdf-ohne-textlayer-wird-leer-gelesen"
ENTSCHEIDUNG = "belegleser-bleibt-unrepariert-bis-der-waechter-meldet"

# Das Rezept für den Bau, dann nicht jetzt (Entscheidung Punkt 3).
REZEPT = (
    "Rezept, BEVOR du verdrahtest (Python und Rust in EINEM Commit): "
    "Python — im Voll-Scan-Zweig von beleg_writer.lies_beleg_text nicht tesseract über die ganze PDF-Datei "
    "laufen lassen, sondern zuerst mit `pdftoppm -png -r 200` rastern und dann je Seite tesseract lesen, "
    "mit Exit-Prüfung, wie kontoauszug_writer.lies_kontoauszug_pdf (_ocr_tesseract_zeilen, _pdftoppm, "
    "_tesseract_tsv). "
    "Rust — in rust/eingang/src/ocr.rs lies_beleg_text über das vorhandene ocr_alle führen statt über einen "
    "tesseract-Lauf auf der Datei. "
    "Prüfen mit ECHTEM tesseract an einem Scan-PDF (kein gefälschtes subprocess.run), dann diesen Wächter "
    "und das Ticket schließen."
)


def verwendungen(quelle: str, name: str = NAME) -> list[int]:
    """Zeilennummern, an denen `name` in `quelle` verwendet wird (Name, Attribut, from-Import).

    Kommentare, Docstrings und die Definition (`def name`) zählen nicht."""
    zeilen = []
    for knoten in ast.walk(ast.parse(quelle)):
        if isinstance(knoten, ast.Name) and knoten.id == name:
            zeilen.append(knoten.lineno)
        elif isinstance(knoten, ast.Attribute) and knoten.attr == name:
            zeilen.append(knoten.lineno)
        elif isinstance(knoten, ast.ImportFrom) and any(a.name == name for a in knoten.names):
            zeilen.append(knoten.lineno)
    return sorted(zeilen)


def zaehle(wurzel: str, name: str = NAME) -> tuple[list[str], dict[str, list[int]]]:
    """(alle gelesenen .py-Dateien relativ zu `wurzel`, {Datei: Zeilen} der Verwendungen von `name`).

    Ein Syntaxfehler in einer Datei wirft: eine nicht lesbare Datei ist kein stilles Grün."""
    gelesen: list[str] = []
    treffer: dict[str, list[int]] = {}
    for dirpath, dirnamen, dateinamen in os.walk(wurzel):
        dirnamen[:] = sorted(d for d in dirnamen if d != "__pycache__")
        for fn in sorted(dateinamen):
            if not fn.endswith(".py"):
                continue
            pfad = os.path.join(dirpath, fn)
            rel = os.path.relpath(pfad, wurzel)
            with open(pfad, encoding="utf-8") as f:
                zeilen = verwendungen(f.read(), name)
            gelesen.append(rel)
            if zeilen:
                treffer[rel] = zeilen
    return gelesen, treffer


def verstoss(wurzel: str) -> str | None:
    """Die Meldung, wenn unter `wurzel` jemand `lies_beleg_text` verwendet; sonst None."""
    _, treffer = zaehle(wurzel)
    if not treffer:
        return None
    wo = ", ".join(f"{rel}:{z}" for rel, zeilen in sorted(treffer.items()) for z in zeilen)
    return (
        f"{NAME} hat einen Aufrufer unter produkt/: {wo}. "
        f"Der Beleg-Leser liest ein PDF ohne Textebene (reiner Scan) als LEEREN Text — tesseract liest kein "
        f"PDF (Exit 1), der Exit wird ignoriert; ein gescannter Beleg sieht dann aus wie ein Beleg ohne "
        f"Beträge, und der Nutzer verliert Abzüge ohne Meldung. "
        f"Ticket: backlog/taxgraph/{TICKET}.md, Entscheidung: decisions/{ENTSCHEIDUNG}.md. {REZEPT}"
    )


def test_lies_beleg_text_hat_keinen_aufrufer_unter_produkt():
    """Der Wächter. Zuerst die Positivkontrolle an Quelltext-Strings (einmal mit Aufruf, einmal ohne):
    ohne sie wäre "0 Treffer" von "nichts gelesen" nicht zu unterscheiden."""
    mit = "import beleg_writer as BW\ntext, conf = BW.lies_beleg_text(pfad)\n"
    ohne = "import beleg_writer as BW\ntext, conf = BW.extrahiere(roh, bindung)\n"
    assert verwendungen(mit) == [2], "Positivkontrolle: der Zähler sieht einen Aufruf nicht"
    assert verwendungen(ohne) == [], "Positivkontrolle: der Zähler sieht einen Aufruf, wo keiner steht"

    meldung = verstoss(PRODUKT)
    assert meldung is None, meldung


def test_zaehler_erkennt_aufruf_weitergabe_und_import_aber_nicht_text():
    """Die Formen, die zählen, und die, die nicht zählen — die Grenze des Zählers, einzeln."""
    # zählen
    assert verwendungen("x = lies_beleg_text(p)\n") == [1]                          # Name
    assert verwendungen("x = BW.lies_beleg_text(p)\n") == [1]                       # Attribut-Aufruf
    assert verwendungen("pool.submit(BW.lies_beleg_text, p)\n") == [1]              # weitergereicht
    assert verwendungen("from beleg_writer import lies_beleg_text\n") == [1]        # Import
    assert verwendungen("from beleg_writer import lies_beleg_text as lb\n") == [1]  # Import mit Alias
    # zählen nicht
    assert verwendungen("def lies_beleg_text(pfad):\n    return pfad\n") == []      # die Definition
    assert verwendungen("# BW.lies_beleg_text(p)\nx = 1\n") == []                    # Kommentar
    assert verwendungen('def f():\n    """ruft lies_beleg_text(p) nicht auf"""\n') == []   # Docstring
    assert verwendungen("x = lies_kontoauszug_pdf(p)\n") == []                      # anderer Name
    assert verwendungen("x = lies_beleg_text_v2(p)\n") == []                        # Namensvetter


def test_scan_liest_den_echten_baum_und_findet_den_bekannten_aufrufer():
    """Positivkontrolle an den echten Daten: derselbe Zähler findet `lies_kontoauszug_pdf` im Produkt
    (`KW.lies_kontoauszug_pdf(pfad)` in haut/api.py) und liest die Datei, in der `lies_beleg_text` steht.
    Ohne das wäre ein leerer oder falsch gesetzter Scan-Pfad ebenfalls "0 Aufrufer"."""
    gelesen, treffer = zaehle(PRODUKT, "lies_kontoauszug_pdf")
    UNTERGRENZE = 30  # gemessen 2026-10-03: 37 .py-Dateien unter produkt/
    assert len(gelesen) >= UNTERGRENZE, f"nur {len(gelesen)} Dateien unter produkt/ gelesen"
    assert os.path.join("eingang", "beleg_writer.py") in gelesen, "beleg_writer.py wird nicht gelesen"
    assert os.path.join("haut", "api.py") in treffer, (
        f"der bekannte Aufrufer lies_kontoauszug_pdf in haut/api.py wird nicht gefunden: {treffer}")


def test_kuenstlicher_aufrufer_macht_den_waechter_rot_und_die_meldung_nennt_ticket_und_rezept(tmp_path):
    """Dieselbe Prüfung (`verstoss`) an einem Baum mit einem künstlichen Aufrufer: sie meldet Ort, Ticket
    und Rezept. Gegenprobe: derselbe Baum ohne den Aufruf meldet nichts."""
    (tmp_path / "haut").mkdir()
    (tmp_path / "haut" / "beleg_upload.py").write_text(
        "import beleg_writer as BW\n\ndef hoch(pfad):\n    return BW.lies_beleg_text(pfad)\n", encoding="utf-8")
    meldung = verstoss(str(tmp_path))
    assert meldung is not None, "ein Aufrufer wurde nicht gemeldet"
    assert os.path.join("haut", "beleg_upload.py") + ":4" in meldung, meldung      # Ort
    assert TICKET in meldung and ENTSCHEIDUNG in meldung, meldung                 # Ticket
    for teil in ("pdftoppm", "lies_kontoauszug_pdf", "ocr_alle", "tesseract"):    # Rezept, beide Sprachen
        assert teil in meldung, f"{teil} fehlt im Rezept: {meldung}"

    (tmp_path / "haut" / "beleg_upload.py").write_text(
        "import beleg_writer as BW\n\ndef hoch(pfad):\n    return BW.extrahiere(pfad, {})\n", encoding="utf-8")
    assert verstoss(str(tmp_path)) is None
