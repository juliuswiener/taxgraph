"""__helpers — geteilte Helfer für konsistenz/ (keine Duplikate mehr).

Die Regel „vorlaeufig zählt nicht als Beleg" ist eine INHALTLICHE Regel, die an genau
einer Stelle kodiert sein darf — zwei Kopien (flag_check.py + partner_check.py) sind
ein Drift-Risiko. check_pauschalen.py hatte die Regel zweimal inline (Z.47, 56).

(Keine dokumentierte Trennungs-Entscheidung für konsistenz/ — im Unterschied zu
beleg_writer/kontoauszug_writer, die mit „geteiltes Muster, eigene Kopie" bewusst
getrennt sind.)
"""
from __future__ import annotations


def _bestaetigt_wert(snapshot: dict, feld_id: str):
    """Wert eines Felds nur, wenn es bestätigt vorliegt (sonst None — vorlaeufig zählt nicht als Beleg).

    None ist zweideutig und trennt NICHT, woher es kommt: Feld ganz abwesend im Snapshot (nie
    beantwortet) und Feld vorhanden mit zustand != "bestaetigt" (z. B. "vorlaeufig", Nutzer mitten
    im Dialog) liefern beide None. Wer die beiden Fälle unterscheiden muss, prüft `feld_id in
    snapshot` selbst VOR dem Aufruf hier — nicht am Rückgabewert dieser Funktion.
    """
    f = snapshot.get(feld_id)
    if f is None or f.get("zustand") != "bestaetigt":
        return None
    return f.get("wert")