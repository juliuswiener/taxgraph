"""Der Dialog duzt durchgehend.

Befund 2026-08-14 beim Durchsehen der Fragetexte (Anlass: Julius' "die Fragen sind zum Teil sehr
fachlich"): 209 Texte duzten, 10 siezten. Betroffen waren zusammenhängende Blöcke — die fünf
§ 36-Anrechnungsfelder, beide Kirchensteuer-Fragen, § 22 Nr. 3 und die § 35a-Mitveranlagung. Also
kein Ausrutscher, sondern Dateien, die zu unterschiedlichen Zeiten entstanden sind.

Für den Nutzer ist der Wechsel mitten im Fragebogen ein Bruch, und zwar einer, den man nicht
begründen kann: dieselbe Software spricht ihn auf derselben Seite anders an.

Warum nur die Anrede geprüft wird und nicht "zu fachlich": Verständlichkeit ist eine
Ermessensfrage, die Anrede nicht. Ein Test, der Fachbegriffe verbietet, würde entweder zu viel
durchlassen oder legitime Begriffe blockieren (Kirchensteuer, Lohnsteuerbescheinigung und
Gewerbesteuer-Messbescheid MÜSSEN so heißen, sonst findet man sie im Bescheid nicht wieder).

NULL LLM.
"""
from __future__ import annotations

import glob
import os
import re

import pytest

yaml = pytest.importorskip("yaml")

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Höflichkeitsform. "Sie" nur als eigenständiges Wort mit großem S — "sie" (Pronomen) und
# Wortteile wie "diese"/"Siedlung" bleiben außen vor.
SIEZEN = re.compile(r"\b(Sie|Ihnen|Ihre|Ihren|Ihrem|Ihrer|Ihres|Ihr)\b")


def _askable_texte():
    """[(feld_id, feldname, text)] über alle Bindungen — Fragetext UND Kurzhilfe."""
    out = []
    for fp in sorted(glob.glob(os.path.join(ROOT, "produkt", "bindung", "bindung_*.yaml"))):
        for b in yaml.safe_load(open(fp)).get("bindungen", []):
            if not b.get("askable"):
                continue
            for schluessel in ("fragetext_laie", "hilfe_kurz"):
                t = b.get(schluessel)
                if t:
                    out.append((b["feld_id"], schluessel, t))
    return out


def test_kein_fragetext_siezt():
    treffer = [(fid, k, SIEZEN.search(t).group(0), t[:70])
               for fid, k, t in _askable_texte() if SIEZEN.search(t)]
    assert not treffer, (
        "Diese Texte siezen, der Rest des Dialogs duzt:\n  "
        + "\n  ".join(f"{fid} ({k}): «{w}» — {t}…" for fid, k, w, t in treffer))


def test_der_test_wuerde_ein_siezen_auch_finden():
    """Selbstprobe: die Regex muss die Höflichkeitsform treffen und das Pronomen 'sie' in Ruhe
    lassen. Ohne das wäre der Test oben womöglich nur deshalb grün, weil er nichts sieht."""
    assert SIEZEN.search("Wie viel haben Sie gezahlt?")
    assert SIEZEN.search("Steht auf der Bescheinigung Ihrer Bank")
    assert not SIEZEN.search("Wenn sie höher ist, gilt der Betrag")
    assert not SIEZEN.search("Diese Angabe steht im Bescheid")
