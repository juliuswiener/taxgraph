"""K2-Flag-Konsistenz — Abwesenheits-Flag ↔ echte Einkunftsart-Felder (Task #11, Front V+V). NULL LLM.

Die an_gesamt-Abwesenheits-Flags (kein_gewinn/kein_kap/kein_vuv/kein_sonstige) behaupten die ABWESENHEIT
einer Einkunftsart (kein_X=true ⇒ „diese Einkunftsart liegt nicht vor"). Sind sie GESETZT und trotzdem
ein echtes Einkunfts-Feld dieser Art mit einem Wert > 0 belegt, ist das ein WIDERSPRUCH — der reine-AN-
Ring würde eine Einkunftsart still übergehen (falscher Bescheid). Dieser Guard surft den Widerspruch als
benannte Inkonsistenz (K2: nie still eine Einkunftsart schlucken).

Beispiel V+V (die Ring-relevante Inversion): `kein_vuv=true` + `vv_einnahmen>0` bestätigt → Widerspruch;
für einen echten V+V-Fall muss `kein_vuv=false` sein (dann greift der V+V-Ring statt der Sperre).
"""
from __future__ import annotations

# Flag (behauptet Abwesenheit) -> die echten Einkunfts-Betragsfelder derselben Art (§ 2 Abs. 1).
FLAG_NEGIERT = {
    "kein_kap":      ["kap_kapitalertraege", "kap_gewinn_aktien"],          # § 2 Abs. 1 Nr. 5
    "kein_vuv":      ["vv_einnahmen"],                                       # § 2 Abs. 1 Nr. 6
    "kein_sonstige": ["rentner_jahresrente"],                               # § 2 Abs. 1 Nr. 7 (Renten)
    "kein_gewinn":   ["einkuenfte_gewinn"],                                  # § 2 Abs. 1 Nr. 1-3 (§§ 13-18 Stufe 1)
}


def _bestaetigt_wert(snapshot: dict, feld_id: str):
    """Wert eines Felds nur, wenn es bestätigt vorliegt (sonst None — vorlaeufig zählt nicht als Beleg)."""
    f = snapshot.get(feld_id)
    if f is None or f.get("zustand") != "bestaetigt":
        return None
    return f.get("wert")


def flag_widersprueche(snapshot: dict) -> list:
    """{feld_id -> {wert, zustand, ...}} → Liste der Flag↔Einkunftsart-Widersprüche.
    Ein Widerspruch: das Flag ist bestätigt=true (Abwesenheit behauptet) UND ein negiertes Einkunfts-
    Feld ist bestätigt mit Wert > 0. Rein deterministisch, kein Rate-Wert."""
    widersprueche = []
    for flag, felder in FLAG_NEGIERT.items():
        if _bestaetigt_wert(snapshot, flag) is not True:
            continue                                    # Flag nicht bestätigt-true -> keine Behauptung
        for feld_id in felder:
            wert = _bestaetigt_wert(snapshot, feld_id)
            if isinstance(wert, (int, float)) and wert > 0:
                widersprueche.append({
                    "flag": flag, "feld_id": feld_id, "wert": wert,
                    "grund": f"{flag}=true behauptet Abwesenheit, aber {feld_id}={wert} liegt vor "
                             f"— für diese Einkunftsart muss das Flag false sein (sonst still übergangen)."})
    return widersprueche
