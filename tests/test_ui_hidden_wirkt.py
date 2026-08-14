"""Das HTML-Attribut `hidden` muss in der UI tatsächlich verstecken.

Befund 2026-08-14, beim ersten Durchklicken durch die eigene Oberfläche: der KI-Berater lag
dauerhaft im Vordergrund, der Start-Screen darunter war nicht anklickbar, und der Schließen-Knopf
tat nichts.

Ursache: `hidden` wirkt nur über den User-Agent-Style `[hidden]{display:none}`. Jede Regel aus
dem Autoren-Stylesheet, die `display` setzt, gewinnt dagegen — hier `.overlay{display:grid}`
zusammen mit `position:fixed; inset:0; z-index:10`. Das Overlay lag also über allem, obwohl im
HTML `hidden` stand, und `chat-zu` (setzt hidden=true) blieb wirkungslos.

Betroffen waren chat-overlay, kette-overlay und ring. Fix: `[hidden]{display:none !important}`
in style.css.

Der Test prüft die Ursache statt des Symptoms — er braucht keinen Browser: für jedes Element mit
hidden-Attribut wird geprüft, ob eine CSS-Regel auf seiner id oder einer seiner Klassen `display`
setzt, und ob die globale [hidden]-Regel existiert, die das aufhebt.

NULL LLM.
"""
from __future__ import annotations

import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
STATIC = os.path.join(ROOT, "produkt", "haut", "static")
HTML = os.path.join(STATIC, "index.html")
CSS = os.path.join(STATIC, "style.css")


def _css() -> str:
    """style.css OHNE Kommentare — der Kommentar zu dieser Regel zitiert `[hidden]{display:none}`
    im Fließtext, und ein naiver Scan fände den Zitattext statt der Regel."""
    return re.sub(r"/\*.*?\*/", "", open(CSS, encoding="utf-8").read(), flags=re.S)


def _hidden_elemente(html: str) -> dict[str, list[str]]:
    """{element_id: [css-klassen]} für jedes Element mit hidden-Attribut."""
    out: dict[str, list[str]] = {}
    for tag in re.finditer(r"<\w+[^>]*>", html):
        t = tag.group(0)
        if not re.search(r"\bhidden\b", t):
            continue
        i = re.search(r'id="([\w-]+)"', t)
        if not i:
            continue
        cl = re.search(r'class="([^"]+)"', t)
        out[i.group(1)] = cl.group(1).split() if cl else []
    return out


def test_hidden_regel_existiert():
    """Die eine Zeile, die das Problem strukturell löst. Ohne sie hängt die Sichtbarkeit jedes
    hidden-Elements davon ab, ob zufällig jemand eine display-Regel auf seine Klasse geschrieben
    hat — das ist keine Eigenschaft, auf die man UI-Zustände stützen kann."""
    css = _css()
    treffer = re.search(r"\[hidden\]\s*\{([^}]*)\}", css)
    assert treffer, (
        "style.css hat keine [hidden]-Regel. Ohne sie überstimmt jede Autoren-Regel mit "
        "display das hidden-Attribut (User-Agent-Style verliert gegen Autoren-Stylesheet).")
    inhalt = treffer.group(1).replace(" ", "")
    assert "display:none" in inhalt, f"[hidden] setzt kein display:none: {treffer.group(1)!r}"
    assert "!important" in inhalt, (
        "[hidden]{display:none} ohne !important verliert erneut gegen jede spätere Regel mit "
        "gleicher Spezifität — .overlay{display:grid} steht in derselben Datei.")


def test_kein_hidden_element_wird_von_einer_display_regel_ueberstimmt():
    """Gegenprobe auf Elementebene: hier stünde die Liste der Elemente, die ohne die globale
    Regel sichtbar blieben. Sie ist der Grund, warum die Regel oben nicht wieder verschwinden
    darf — beim Fund waren es chat-overlay, kette-overlay und ring."""
    html = open(HTML, encoding="utf-8").read()
    css = _css()
    # die globale Regel absichtlich ausklammern, um die darunterliegende Lage zu messen
    css_ohne_global = re.sub(r"\[hidden\]\s*\{[^}]*\}", "", css)

    gefaehrdet = []
    for eid, klassen in _hidden_elemente(html).items():
        for sel in [f"#{eid}"] + [f".{k}" for k in klassen]:
            for m in re.finditer(re.escape(sel) + r"\s*\{([^}]*)\}", css_ohne_global):
                if re.search(r"display\s*:", m.group(1)):
                    gefaehrdet.append((eid, sel))
                    break

    # Kein Verbot — nur der Nachweis, dass die globale Regel gebraucht wird und wirkt.
    assert re.search(r"\[hidden\]\s*\{[^}]*display\s*:\s*none[^}]*!important", css), (
        f"Ohne globale [hidden]-Regel blieben diese Elemente sichtbar: {gefaehrdet}")
