"""Kein Fremdtext als Markup — ausnahmslos, und deshalb prüfbar.

Der Fund (Audit 2026-08-16, sec-xss-innerhtml-rohtext), 2026-08-17 vor der Reparatur
nachgesehen: in `app.js:herkunftKette` baute ein Helfer seine Zeile per Interpolation

    d.innerHTML = `<span class="dot">${dot}</span> <b>${titel}</b> ${txt ? "· " + txt : ""}`;

und wurde unter anderem so gerufen:

    step("▤", "Beleg", `... ${s1.roh_text} ...`)

`roh_text` ist der OCR-Rohtext eines hochgeladenen Belegs. Ein Beleg schreibt aber nicht der
Nutzer, sondern wer ihm die Rechnung geschickt hat — präpariertes Markup lief damit im Kontext
der Anwendung. Besonders unangenehm, seit das Anmelde-Token in sessionStorage liegt: dessen
Wahl ist im Login-Commit ausdrücklich MIT diesem XSS-Fenster begründet worden.

WARUM STRUKTURELL STATT PUNKTUELL: die eine Zeile war in Minuten repariert. Aber ein
Verhaltenstest deckt nur den Sink ab, den er kennt — der nächste entsteht beim nächsten
Feature, und Fremdtext kommt in dieser Anwendung an vielen Stellen an (OCR, Kontoauszug,
KI-Antworten, Zitatanker). Deshalb hier die Regel über die ganze Datei.

WARUM OHNE AUSNAHMELISTE: es gab genau eine weitere Interpolation (die Ergebniszahl, aus
eigener Rechnung, unbedenklich). Sie ist trotzdem mit umgestellt worden, statt sie
auszunehmen. Eine Liste "diese Stelle ist harmlos" verrottet, und die nächste Stelle wird nach
ihrem Vorbild gebaut. Ausnahmslos ist die einzige Fassung, die man in einem Jahr noch glaubt.

Erlaubt bleibt `innerHTML = ""` (Leeren eines Containers) — dabei entsteht kein Markup aus
Daten. Erlaubt bleiben auch statische Literale ohne Interpolation, etwa das Formular der
Maps-Affordanz: dort steht kein einziger Wert aus einer Antwort.

Zweite Verteidigungslinie ist der CSP-Header (tests/test_idor_und_csp.py) — der greift, falls
doch einmal ein Sink durchrutscht. Beide zusammen, nicht eines statt des anderen.

NULL LLM, kein Browser nötig.
"""
from __future__ import annotations

import glob
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
STATIC = os.path.join(ROOT, "produkt", "haut", "static")

# innerHTML-Zuweisung, deren rechte Seite eine Interpolation `${...}` enthält.
_SINK = re.compile(r'\.innerHTML\s*=\s*[^;\n]*\$\{')
# `+`-Verkettung auf innerHTML ist derselbe Fehler in anderer Schreibweise.
_SINK_PLUS = re.compile(r'\.innerHTML\s*(\+)?=\s*[^;\n]*["\'`]\s*\+\s*[A-Za-z_$]')


def _js_dateien() -> list[str]:
    return sorted(glob.glob(os.path.join(STATIC, "*.js")))


def _ist_kommentar(zeile: str) -> bool:
    """Zeilen-Kommentare überspringen. Ein Kommentar führt kein Markup aus — und genau hier
    zitieren mehrere Kommentare die ALTE, unsichere Zeile, um zu erklären, warum sie weg ist.
    Ohne diese Ausnahme müsste man die Erklärung löschen, um das Gate grün zu bekommen; dann
    stünde da irgendwann wieder unsicherer Code, weil niemand mehr weiß, warum nicht."""
    return zeile.lstrip().startswith(("//", "*", "/*"))


def test_keine_innerhtml_interpolation():
    """Der Kern. Wer Text anzeigen will, nimmt textContent; wer Struktur braucht, baut sie
    mit createElement. Beides kann Markup nicht ausführen."""
    treffer = []
    for pfad in _js_dateien():
        for nr, zeile in enumerate(open(pfad, encoding="utf-8"), 1):
            if _ist_kommentar(zeile):
                continue
            if _SINK.search(zeile) or _SINK_PLUS.search(zeile):
                treffer.append(f"{os.path.basename(pfad)}:{nr}: {zeile.strip()[:110]}")
    assert not treffer, (
        "innerHTML mit interpoliertem/verkettetem Inhalt — Fremdtext kann dort als Markup "
        "ausgeführt werden (OCR-Rohtext, Kontoauszug, KI-Antwort):\n  " + "\n  ".join(treffer))


def test_die_regel_erkennt_ihren_eigenen_fehlerfall():
    """Negativprobe des Gates: ohne sie wäre nicht belegt, dass das Muster überhaupt greift —
    ein Gate, das seinen eigenen Fehlerfall nicht kennt, ist eine Behauptung. Beide
    Schreibweisen, weil die zweite sonst am ersten Muster vorbeirutscht."""
    assert _SINK.search('d.innerHTML = `<b>${titel}</b>`;')
    assert _SINK_PLUS.search('d.innerHTML = "<b>" + titel;')
    assert _SINK_PLUS.search('d.innerHTML += "<i>" + txt;')
    # Und die erlaubten Fälle dürfen NICHT anschlagen, sonst ist die Regel unbenutzbar:
    assert not _SINK.search('ul.innerHTML = "";')
    assert not _SINK_PLUS.search('ul.innerHTML = "";')
    assert not _SINK.search('wrap.innerHTML = `<div class="maps-titel">Entfernung</div>`;')


def test_der_geheilte_sink_ist_wirklich_umgebaut():
    """Punktuell zusätzlich zur Regel: der konkrete Fund von 2026-08-17. Die Regel oben würde
    auch dann grün, wenn jemand `herkunftKette` ersatzlos löschte — dieser Test verlangt, dass
    die Beleg-Zeile weiterhin GEBAUT wird, nur eben sicher."""
    quelle = open(os.path.join(STATIC, "app.js"), encoding="utf-8").read()
    assert "roh_text" in quelle, "die Beleg-Herkunft wird gar nicht mehr angezeigt?"
    assert "const step = (dot, titel, txt)" in quelle, "step-Helfer nicht mehr da?"
    # textContent/createElement statt Markup-Interpolation im Helfer.
    abschnitt = quelle.split("const step = (dot, titel, txt)")[1][:600]
    assert "textContent" in abschnitt, f"step baut nicht mehr per textContent:\n{abschnitt[:300]}"
    assert "innerHTML" not in abschnitt, f"step benutzt wieder innerHTML:\n{abschnitt[:300]}"
