"""K2-Pauschal-Konsistenz — Vergessene Pauschalen (Sparer-PB, EP-Arbeitstage, V+V-WK). NULL LLM.

Prüft, ob ein Steuerpflichtiger Einkunfts-Quellen hat, die eine Standard-Pauschale auslösen,
aber das zugehörige Mengen-/Param-Feld fehlt oder 0 ist. Das sind SOFTE Hinweise (keine harten
Widersprüche wie bei flag/partner): der Nutzer könnte eine vergessene Angabe nachreichen und
damit Steuern sparen.

Jeder Check definiert:
  - ausloeser_felder: Einkunfts-Felder, deren bestätigter Wert > 0 die Pauschale relevant macht
  - pauschal_felder: die Felder, die der Nutzer ausfüllen müsste, um die Pauschale zu aktivieren
"""
from __future__ import annotations

from typing import Any

PAUSCHAL_CHECKS: tuple[dict[str, Any], ...] = (
    {
        "id": "sparer_pb",
        "ausloeser_felder": ("kap_kapitalertraege", "kap_gewinn_aktien"),
        "pauschal_felder": ("kap_zusammenveranlagung",),
        "label": "Sparer-Pauschbetrag (§ 20 Abs. 9)",
        "hinweis": "Kapitaleinkünfte vorhanden, aber der Sparer-Pauschbetrag (1.000/2.000 €) ist "
                    "nur mit Angabe der Veranlagungsart korrekt bestimmbar.",
    },
    {
        "id": "ep_arbeitstage",
        "ausloeser_felder": ("bruttoarbeitslohn",),
        "pauschal_felder": ("ep_arbeitstage",),
        "label": "Entfernungspauschale (EP-Arbeitstage)",
        "hinweis": "Arbeitslohn vorhanden, aber keine Anzahl an Arbeitstagen für die "
                    "Entfernungspauschale angegeben. Möglicherweise wurde die Pauschale vergessen.",
    },
    {
        "id": "vv_wk",
        "ausloeser_felder": ("vv_einnahmen",),
        "pauschal_felder": ("vv_schuldzinsen", "vv_erhaltungsaufwand", "vv_sonstige_wk"),
        "label": "Werbungskosten bei Vermietung und Verpachtung (§ 21)",
        "hinweis": "Einnahmen aus Vermietung vorhanden, aber keine Werbungskosten erfasst. "
                    "Möglicherweise wurden Ausgaben (Schuldzinsen, Erhaltungsaufwand, etc.) vergessen.",
    },
)


def _bestaetigt_wert_gt0(snapshot: dict, feld_id: str) -> bool:
    """True wenn Feld bestätigt und Wert > 0."""
    f = snapshot.get(feld_id)
    if f is None or f.get("zustand") != "bestaetigt":
        return False
    w = f.get("wert")
    return isinstance(w, (int, float)) and not isinstance(w, bool) and w > 0


def _pauschal_feld_ist_leer(snapshot: dict, feld_id: str) -> bool:
    """True wenn Feld fehlt, unbestätigt oder Wert == 0/False/None."""
    f = snapshot.get(feld_id)
    if f is None or f.get("zustand") != "bestaetigt":
        return True
    w = f.get("wert")
    # 0, False, None, "" gelten als leer
    if w is None or w is False or w == 0:
        return True
    if isinstance(w, str) and not w.strip():
        return True
    return False


def pauschal_hinweise(snapshot: dict) -> list[dict]:
    """Snapshot → Liste der Pauschal-Hinweise (leer wenn keine vergessene Pauschale).

    Jeder Hinweis:
      - check_id: z.B. "sparer_pb"
      - label: lesbarer Name der Pauschale
      - hinweis: Erklärung für UI
      - ausloeser_felder: welche income-Felder den Check ausgelöst haben (name + wert)
      - fehlende_felder: welche Pauschal-Felder leer sind
    """
    hinweise = []
    for check in PAUSCHAL_CHECKS:
        ausloeser = []
        for fid in check["ausloeser_felder"]:
            f = snapshot.get(fid)
            if _bestaetigt_wert_gt0(snapshot, fid):
                ausloeser.append({"feld_id": fid, "wert": f["wert"]})
        if not ausloeser:
            continue  # keine Einkunfts-Quelle → Pauschale nicht relevant

        fehlende = []
        for fid in check["pauschal_felder"]:
            if _pauschal_feld_ist_leer(snapshot, fid):
                fehlende.append(fid)
        if not fehlende:
            continue  # alle Pauschal-Felder ausgefüllt → kein Hinweis
        # Für vv_wk: nur warnen wenn ALLE WK-Felder leer sind (nicht nur eines)
        if check["id"] == "vv_wk" and len(fehlende) < len(check["pauschal_felder"]):
            continue

        hinweise.append({
            "check_id": check["id"],
            "label": check["label"],
            "hinweis": check["hinweis"],
            "ausloeser_felder": ausloeser,
            "fehlende_felder": fehlende,
        })
    return hinweise
