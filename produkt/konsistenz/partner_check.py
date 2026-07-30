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

# Partner-Behinderungsfelder, die eine Zusammenveranlagung voraussetzen (§ 33b Person B, Instanz-Reuse
# derselben E0109708/E0109706-Kz wie Person A, kein eigenes Kz — XSD-Kz-Section-Sweep Cluster C).
PARTNER_FELDER = (
    "rentner_grad_der_behinderung_partner",     # E0109708 (GdB, Instanz-Reuse)
    "rentner_hilflos_blind_taubblind_partner",  # E0109706 (hilflos/blind, Instanz-Reuse)
    # § 20 Kapital Person-B (Anlage-KAP-Instanz B, Klasse g) — nur bei Zusammenveranlagung sinnvoll:
    "kap_kapitalertraege_partner",
    "kap_gewinn_aktien_partner",
    "kap_gewinn_sonstige_partner",   # Register-B-K2-Fix 2026-07-19 (symmetrisch, vorher fehlend)
    "kap_verlust_aktien_partner",
    "kap_verlust_sonstige_partner",
    # § 22 Rente Person-B (Anlage-R-Instanz B, Klasse g×f) — der Renten-Betrag ist der Indikator (die
    # int-Meta beginn/alter würden über _ist_gesetzt (>0) falsch-positiv, daher nur der Betrag):
    "rentner_jahresrente_partner",
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
    PARTNER_NAME = {
        "rentner_grad_der_behinderung_partner": "Grad der Behinderung des Partners",
        "rentner_hilflos_blind_taubblind_partner": "Behinderten-Merkzeichen des Partners",
        "kap_kapitalertraege_partner": "Kapitaleinkünfte des Partners",
        "kap_gewinn_aktien_partner": "Aktiengewinne des Partners",
        "kap_gewinn_sonstige_partner": "sonstige Kapitalgewinne des Partners",
        "kap_verlust_aktien_partner": "Aktienverluste des Partners",
        "kap_verlust_sonstige_partner": "sonstige Kapitalverluste des Partners",
        "rentner_jahresrente_partner": "Rentenbezüge des Partners",
    }
    veranlagung = _bestaetigt_wert(snapshot, "veranlagung")
    if veranlagung is None or veranlagung == "zusammen":
        return []                                   # unbestätigt oder korrekt zusammen -> kein Widerspruch
    widersprueche = []
    for feld_id in PARTNER_FELDER:
        wert = _bestaetigt_wert(snapshot, feld_id)
        if not _ist_gesetzt(wert):
            continue
        feld_titel = PARTNER_NAME.get(feld_id, feld_id)
        widersprueche.append({
            "feld_id": feld_id, "wert": wert, "veranlagung": veranlagung,
            "grund": f"Du hast Angaben zu den {feld_titel} gemacht, aber keine "
                     f"Zusammenveranlagung gewählt. Partnerbezogene Angaben sind nur bei "
                     f"gemeinsamer Veranlagung möglich. Bitte prüfe deine Angaben."})
    return widersprueche


def alleinerziehend_mit_zusammen(snapshot: dict) -> list:
    """{feld_id -> {wert, zustand, ...}} → Liste der § 24b-Alleinerziehend ↔ Veranlagung-Widersprüche.

    INVERSE Richtung zu partner_ohne_zusammen: der Entlastungsbetrag für Alleinerziehende (§ 24b) setzt
    voraus, dass der Steuerpflichtige NICHT die Voraussetzungen der Ehegatten-Zusammenveranlagung erfüllt
    (§ 24b Abs. 1: „allein stehend", § 24b Abs. 3). Ist `fam_alleinstehend` bestätigt True UND `veranlagung`
    bestätigt „zusammen", ist das ein WIDERSPRUCH — der § 24b-Abzug würde sonst still gewährt = Unter-
    Besteuerung. Fail-closed: benannte Inkonsistenz statt stillem Abzug. Nur bestätigte Werte zählen
    (vorlaeufig ist kein Beleg); ist die Veranlagung noch unbestätigt (None), liegt (noch) kein Widerspruch
    vor (Unvollständigkeit, nicht Widerspruch)."""
    veranlagung = _bestaetigt_wert(snapshot, "veranlagung")
    alleinstehend = _bestaetigt_wert(snapshot, "fam_alleinstehend")
    if veranlagung != "zusammen" or alleinstehend is not True:
        return []                                   # nur zusammen + bestätigt-alleinstehend = Widerspruch
    return [{
        "feld_id": "fam_alleinstehend", "wert": True, "veranlagung": veranlagung,
        "grund": "Du hast angegeben, alleinstehend zu sein — aber auch eine gemeinsame "
                 "Veranlagung mit deinem Partner gewählt. Der Entlastungsbetrag für Alleinerziehende "
                 "setzt voraus, dass du nicht zusammenveranlagt bist. Bitte prüfe deine Angaben."}]
