"""an_gesamt §32b Gap-C (K2, Over-tax): Lohnersatz (Elterngeld/Krankengeld) ist bei
Angestellten haeufig, aber an_gesamts catala_est hat kein extrahierbares zvE → GATE
statt stiller Under-tax (progression_gehoert_in_gesamt). Der Guard liest den Rohwert
ABSICHTLICH ohne Zustandspruefung (_positiv): er soll auch auf einen nur vorlaeufigen
Wert anhalten. Wert 0 ist der Normalfall (kein Lohnersatz) und darf NIE sperren.
NULL LLM, kein HTTP — direkter Aufruf von API._an_gesamt_sperrgrund.
"""

import api as API
import api_constants as AC

AN = AC.SCHEIBEN["an_gesamt"]


def _p32b(wert, zustand):
    return {"p32b_progressionseinkuenfte": {"wert": wert, "zustand": zustand}}


def test_progressionseinkuenfte_vorlaeufig_sperrt():
    """Vorlaeufiger Lohnersatz > 0 → sperrt (Rohwert, keine Zustandspruefung)."""
    assert API._an_gesamt_sperrgrund(_p32b(120000, "vorlaeufig"), AN) == "progression_gehoert_in_gesamt"


def test_progressionseinkuenfte_bestaetigt_sperrt():
    """Bestaetigter Lohnersatz > 0 → sperrt."""
    assert API._an_gesamt_sperrgrund(_p32b(120000, "bestaetigt"), AN) == "progression_gehoert_in_gesamt"


def test_progressionseinkuenfte_null_sperrt_nicht():
    """Wert 0 (kein Lohnersatz) → kein Sperrgrund. Auf das ERWARTETE Ergebnis pruefen
    (is None), nie auf != "progression_gehoert_in_gesamt" — sonst wuerde ein fremder
    Sperrgrund den Kontrastfall gruen faerben."""
    assert API._an_gesamt_sperrgrund(_p32b(0, "bestaetigt"), AN) is None
