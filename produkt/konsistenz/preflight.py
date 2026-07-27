"""K2-Preflight-Orchestrator — Sammelt alle Konsistenz-Prüfungen vor Submission. NULL LLM.

Führt nacheinander aus:
  1. flag_check.flag_widersprueche()  — Abwesenheits-Flag ↔ Einkunftsart
  2. partner_check.partner_ohne_zusammen() + alleinerziehend_mit_zusammen()  — Partner-Feld ↔ Veranlagung
  3. check_pauschalen.pauschal_hinweise()  — Vergessene Pauschalen (soft)

Ergebnis: dict mit den drei Ergebnissen + aggregiertem status (RED/AMBER/GREEN).
"""
from __future__ import annotations

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                                "konsistenz"))
import flag_check       # noqa: E402
import partner_check    # noqa: E402
import check_pauschalen  # noqa: E402


def preflight(snapshot: dict) -> dict:
    """Snapshot → Preflight-Ergebnis.

    Rückgabe:
      - widersprueche_flag: Liste (flag_check)
      - widersprueche_partner: Liste (partner_ohne_zusammen)
      - widersprueche_alleinerziehend: Liste (alleinerziehend_mit_zusammen)
      - hinweise_pauschalen: Liste (check_pauschalen)
      - status: "RED" (harte Widersprüche), "AMBER" (nur soft warnings), "GREEN" (clean)
    """
    flag = flag_check.flag_widersprueche(snapshot)
    partner = partner_check.partner_ohne_zusammen(snapshot)
    alleinerziehend = partner_check.alleinerziehend_mit_zusammen(snapshot)
    pauschal = check_pauschalen.pauschal_hinweise(snapshot)

    if flag or partner or alleinerziehend:
        status = "RED"
    elif pauschal:
        status = "AMBER"
    else:
        status = "GREEN"

    return {
        "widersprueche_flag": flag,
        "widersprueche_partner": partner,
        "widersprueche_alleinerziehend": alleinerziehend,
        "hinweise_pauschalen": pauschal,
        "status": status,
    }
