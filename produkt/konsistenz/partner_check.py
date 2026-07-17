"""K2-Partner-Konsistenz — Partner-Behinderungsfeld ↔ Zusammenveranlagung (§ 26b). NULL LLM.

Die Ehegatte-Behinderungsfelder (rentner_*_partner, § 33b für Person B) sind NUR bei Zusammenveranlagung
sinnvoll: ein eigener Behinderten-Pauschbetrag des Partners setzt voraus, dass der Partner steuerlich Teil
DIESER Erklärung ist (§ 26b). Ist ein Partner-Feld GESETZT, die Veranlagung aber bestätigt „einzel"
(bzw. ungleich „zusammen"), ist das ein WIDERSPRUCH — ein Einzelveranlagter hat keinen mitzuveranlagenden
Partner. Fail-closed: nie still einen Partner-Wert in eine Einzelveranlagung durchreichen.

Spiegelt den Haut-Runtime-Guard `partner_kegel_offen` (api.py) in die inverse Richtung: dort „zusammen +
Partner-Pflichtfeld offen → kein halber Bescheid", hier „Partner-Feld gesetzt + nicht zusammen → benannte
Inkonsistenz". Rein deterministisch (nur bestätigte Werte zählen, kein Rate-Wert), analog flag_check.
"""
from __future__ import annotations

# Partner-Behinderungsfelder, die eine Zusammenveranlagung voraussetzen (§ 33b Person B, E05058-Block).
PARTNER_FELDER = (
    "rentner_grad_der_behinderung_partner",     # E0505809 (GdB Ehegatte)
    "rentner_hilflos_blind_taubblind_partner",  # E0505807 (hilflos/blind Ehegatte)
)


def _bestaetigt_wert(snapshot: dict, feld_id: str):
    """Wert eines Felds nur, wenn es bestätigt vorliegt (sonst None — vorlaeufig zählt nicht als Beleg)."""
    f = snapshot.get(feld_id)
    if f is None or f.get("zustand") != "bestaetigt":
        return None
    return f.get("wert")


def _ist_gesetzt(wert) -> bool:
    """Partner-Feld gilt als 'gesetzt', wenn GdB > 0 (int) oder Merkzeichen-Flag True (bool)."""
    if wert is True:
        return True
    return isinstance(wert, (int, float)) and not isinstance(wert, bool) and wert > 0


def partner_ohne_zusammen(snapshot: dict) -> list:
    """{feld_id -> {wert, zustand, ...}} → Liste der Partner↔Veranlagung-Widersprüche.
    Ein Widerspruch: ein Partner-Feld ist bestätigt gesetzt UND veranlagung ist bestätigt ungleich
    „zusammen". Ist die Veranlagung noch unbestätigt (None), liegt (noch) kein Widerspruch vor —
    das ist Unvollständigkeit, die der Kegel/Vollständigkeits-Pfad behandelt, nicht dieser Guard."""
    veranlagung = _bestaetigt_wert(snapshot, "veranlagung")
    if veranlagung is None or veranlagung == "zusammen":
        return []                                   # unbestätigt oder korrekt zusammen -> kein Widerspruch
    widersprueche = []
    for feld_id in PARTNER_FELDER:
        wert = _bestaetigt_wert(snapshot, feld_id)
        if not _ist_gesetzt(wert):
            continue
        widersprueche.append({
            "feld_id": feld_id, "wert": wert, "veranlagung": veranlagung,
            "grund": f"{feld_id}={wert} setzt Zusammenveranlagung voraus (§ 26b), aber "
                     f"veranlagung={veranlagung!r} — ein Partner-Behinderten-Pauschbetrag ist ohne "
                     f"gemeinsam veranlagten Ehe-/Lebenspartner ausgeschlossen (§ 33b)."})
    return widersprueche
