"""Angaben, die das Finanzamt bekommt, unsere Berechnung aber nicht kennt.

Eine eigene Kategorie neben den Widersprüchen und den vergessenen Pauschalen, weil sie etwas
anderes sagt: hier ist nichts falsch und nichts vergessen — die Erklärung ist vollständig und
korrekt. Nur unsere ANGEZEIGTE Zahl bildet einen Teil davon nicht ab.

Warum das überhaupt vorkommt: das Produkt deklariert mehr Tatbestände, als der Rechenkern
abbildet. Das ist der bewusste Zuschnitt — eine Erklärung, die durchgeht, ist mehr wert als
eine, die an einem seltenen Sonderfall scheitert. Der Preis ist, dass die Vorschau in genau
diesen Fällen zu hoch liegt. Das ist die ungefährliche Richtung (der Bescheid fällt günstiger
aus, nicht teurer), aber der Nutzer soll nicht raten müssen, warum die Zahlen auseinandergehen.

NULL LLM, keine Seiteneffekte, reine Funktion über dem Snapshot.
"""
from __future__ import annotations

# feld_id -> Text. Bewusst eine Tabelle und keine verstreuten ifs: wer hier einen Eintrag
# hinzufügt, hat gerade ein Feld gebunden, das der Ring nicht liest — und soll beim Schreiben
# des Hinweises merken, dass das eine Aussage über eine Lücke ist, nicht über den Nutzer.
NICHT_GERECHNET = {
    "spenden_vermoegensstock": (
        "Deine Spende in den Vermögensstock einer Stiftung steht in der Erklärung ans "
        "Finanzamt, wird in der hier angezeigten Steuer aber noch nicht berücksichtigt "
        "(§ 10b Abs. 1a EStG). Dein Bescheid kann deshalb günstiger ausfallen als die "
        "Vorschau."
    ),
}


def nicht_gerechnete_angaben(snapshot: dict) -> list[dict]:
    """Felder aus NICHT_GERECHNET, die bestätigt und größer null sind.

    Nur bestätigte Werte: ein vorläufiger Entwurf ist noch keine Angabe, und ein Hinweis darauf
    wäre Rauschen. Nur > 0: die Antwort "0" ist der Normalfall und sagt gerade, dass der
    Sonderfall nicht vorliegt — dafür gibt es nichts zu erklären.
    """
    treffer = []
    for feld_id, text in NICHT_GERECHNET.items():
        eintrag = snapshot.get(feld_id)
        if not isinstance(eintrag, dict):
            continue
        if eintrag.get("zustand") != "bestaetigt":
            continue
        wert = eintrag.get("wert")
        if isinstance(wert, int) and not isinstance(wert, bool) and wert > 0:
            treffer.append({"feld_id": feld_id, "hinweis": text})
    return treffer
