"""Vollständigkeits-Gate über alle Ring-Feature-Gruppen — Backlog
verpflegungskuerzung-fehlt-in-scheibe-an-gesamt.md, Abschnitt "Gate (Vorschlag)",
umgesetzt als dritter Commit dieses Eintrags.

WAS DAS GATE PRÜFT
`_mit_ring_werten()` (produkt/bescheid/bescheid_deklaration.py) rechnet zehn Gruppen von
Feldern aus und hängt sie als fertige Events in `felder`. Damit diese Werte die Deklaration
überhaupt erreichen, muss die SCHEIBE sie führen: `_scheibe_bindung()` (api.py:179) filtert
die Bindung auf `cfg["felder"]`, und ein Ergebnisfeld, das dort nicht steht, wird
stillschweigend fallengelassen. Der Ring rechnet dann, das XML zeigt es aber nicht.

Genau diese Naht hat zweimal Geld gekostet (an_gesamt/Verpflegungskürzung und
rentner_gesamt/GewSt). Beide sind inzwischen repariert; das Gate hält sie und die acht
anderen Gruppen zu.

WARUM JEDES PAAR EINZELN UND NICHT ALS SAMMEL-ASSERT
Ein Sammel-Assert über alle Gruppen zusammen wird grün, sobald eine einzige Zeile kaputt
ist — genau die Blindheit, an der die Vorgängerzählung ("2 von 12") scheiterte. Jede
(Scheibe, Gruppe) ist darum ein eigener Testfall (`@parametrize`), und die Fehlermeldung
nennt Scheibe, Gruppe und die fehlenden Felder namentlich.

DIE AUSNAHMELISTE (Ratsche, darf NUR schrumpfen)
Die Vorgabe verlangt: weitere Lücken NICHT reparieren, sondern als explizite Ausnahmeliste
mit Längen-Assert eintragen, je Lücke eine Zeile Begründung. Heute ist die Liste LEER —
nachgemessen am 2026-09-26: alle zehn Gruppen sind auf allen Scheiben vollständig.

WAS DAS GATE NICHT SIEHT
- Ob ein Ergebnisfeld den richtigen WERT trägt (nur, dass es in der Scheibe steht). Den Wert
  deckt je Gruppe ein eigener Test ab (z.B. test_an_gesamt_verpflegungskuerzung_senkt_die_zahl).
- Ob die Trigger-Felder ihrerseits erreichbar (askable) sind — das prüft keiner der beiden
  Zustände hier; es ist die Lücke aus [[scheibe-ohne-kachel-haelt-felder-unerreichbar]].
- Ob es Ring-Ergebnisfelder AUSSERHALB dieser zehn Gruppen gibt, die nach demselben Muster
  fehlen. Die zehn sind die vollständige Liste aus dem Docstring von `_mit_ring_werten`;
  ein elfter Bau wäre hier unsichtbar.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "produkt", "haut"))

import api_constants as AC   # noqa: E402


# ---------------------------------------------------------------------------
# Die zehn Gruppen aus _mit_ring_werten (bescheid_deklaration.py:58-345).
#
# Je Gruppe: (name, trigger_felder, ergebnis_felder). Die Trigger sind die Felder, an denen
# `_mit_ring_werten` erkennt, dass die Gruppe überhaupt einschlägig ist; die Ergebnisfelder
# sind die, die sie daraufhin in `felder` einhängt. Steht ein Trigger in der Scheibe, muss
# jedes Ergebnisfeld es auch — sonst rechnet der Ring ins Leere.
# ---------------------------------------------------------------------------
RING_GRUPPEN = (
    # (1) Verpflegungskürzung § 9 Abs. 4a
    ("verpflegungskuerzung",
     ("tage_24h", "tage_an_abreise", "tage_ueber_8h_eintaegig"),
     ("p9_4a_kuerzung_nach_entgelt",)),
    # (2)+(3) Anlage KAP: Antrag + genutzter Sparer-Pauschbetrag (entstehen zusammen)
    ("kap_antrag",
     ("kap_kapitalertraege", "kap_gewinn_aktien", "kap_verlust_aktien",
      "kap_gewinn_sonstige", "kap_verlust_sonstige",
      "kap_kapitalertraege_partner", "kap_gewinn_aktien_partner",
      "kap_gewinn_sonstige_partner"),
     ("kap_antrag_guenstigerpruefung", "kap_sparer_pauschbetrag_genutzt")),
    # (4) § 35a Haushaltsnahe: Sum-Kz aus der Σ der Einzel-Instanzen
    ("haushalt_35a_summe",
     ("hh_minijob_betrag", "hh_dienstleistung_betrag", "hh_handwerker_betrag"),
     ("hh_minijob_aufwendungen", "hh_dienstleistungen", "hh_handwerker_arbeitskosten")),
    # (5) § 35c: die Umkehr-Antwort E0240902 (Trigger ist das Gate-Feld, nicht die Summe)
    ("p35c_umkehr",
     ("p35c_keine_doppelfoerderung",),
     ("p35c_foerderung_in_anspruch",)),
    # (6) § 35c-Einzelzeile: derselbe Betrag in der Zeile der Maßnahmenart
    ("p35c_einzelzeile",
     ("p35c_sanierungsaufwendungen",),
     ("p35c_massnahme_einzelbetrag",)),
    # (7) Anlage V: Summenzeilen der Mieteinnahmen
    ("vv_summen",
     ("vv_einnahmen",),
     ("vv_mieteinnahmen_summe", "vv_einnahmen_summe_gesamt",
      "vv_summe_werbungskosten", "vv_ueberschuss", "vv_ueberschuss_person_a")),
    # § 35 / § 16 Abs. 1 GewStG: zu zahlende Gewerbesteuer = Messbetrag × Hebesatz
    ("gewst_zu_zahlen",
     ("gewst_hebesatz", "gewst_messbetrag"),
     ("gewst_zu_zahlen",)),
    ("gewst_zu_zahlen_partner",
     ("gewst_hebesatz_partner", "gewst_messbetrag_partner"),
     ("gewst_zu_zahlen_partner",)),
    # § 22 Nr. 3: Einzelposten + Werbungskosten aus Einnahmen/Einkünften.
    # Trigger sind BEIDE Nutzereingaben — der Bau verlangt sie bestätigt (`all(...)`).
    ("p22_nr3",
     ("p22_nr3_einnahmen", "p22_nr3_einkuenfte"),
     ("p22_nr3_einnahmen_einzelbetrag", "p22_nr3_werbungskosten")),
    # § 10 Abs. 1 Nr. 7: Einzelzeile zur Berufsausbildungs-Summe
    # (Trigger ist die vom Nutzer erfragte SUMME, Ergebnis die Einzelzeile)
    ("berufsausbildung_einzelzeile",
     ("berufsausbildung_aufwendungen",),
     ("berufsausbildung_einzelbetrag",)),
)

# Bewusst geparkte Lücken, je (scheibe, gruppe). Eine Zeile Begründung je Eintrag.
# Heute LEER — nachgemessen 2026-09-26: alle zehn Gruppen sind auf allen Scheiben, die sie
# anwenden, vollständig (inkl. der vormals kaputten Paare an_gesamt/Verpflegungskürzung und
# rentner_gesamt/GewSt, beide inzwischen repariert).
# Darf NUR schrumpfen. Ein neuer Eintrag hier braucht eine Begründung, warum das Reparieren
# die falsche Antwort wäre — sonst ist er eine stille Rückkehr derselben Lücke.
RING_LUECKEN_AUSNAHMEN: dict[tuple[str, str], str] = {}


def _scheiben_mit_trigger(trigger: tuple) -> list[str]:
    """Alle Scheiben, deren Feldliste mindestens ein Trigger-Feld der Gruppe führt."""
    treffer = []
    for scheibe, cfg in AC.SCHEIBEN.items():
        felder = set(cfg.get("felder") or ())
        if felder & set(trigger):
            treffer.append(scheibe)
    return sorted(treffer)


def _gruppen_faelle() -> list[tuple[str, str, tuple, tuple]]:
    """(scheibe, gruppe, trigger, ergebnis) fuer jedes anwendbare Paar."""
    faelle = []
    for name, trigger, ergebnis in RING_GRUPPEN:
        for scheibe in _scheiben_mit_trigger(trigger):
            faelle.append((scheibe, name, trigger, ergebnis))
    return faelle


@pytest.mark.parametrize("scheibe,gruppe,trigger,ergebnis",
                         _gruppen_faelle(),
                         ids=lambda v: v if isinstance(v, str) else "")
def test_gruppe_vollstaendig_gebunden(scheibe, gruppe, trigger, ergebnis):
    """Führt die Scheibe einen Trigger, muss sie auch jedes Ergebnisfeld führen.

    Ein Paar pro Testfall — ein grünes Sammelergebnis darf keine kaputte Zeile verdecken.
    """
    if (scheibe, gruppe) in RING_LUECKEN_AUSNAHMEN:
        pytest.skip(f"bewusst geparkt: {RING_LUECKEN_AUSNAHMEN[(scheibe, gruppe)]}")
    felder = set(AC.SCHEIBEN[scheibe]["felder"])
    assert felder & set(trigger), (
        f"Testaufbau kaputt: {scheibe} führt keinen Trigger von {gruppe} — "
        f"dieser Fall sollte gar nicht entstehen."
    )
    fehlend = sorted(set(ergebnis) - felder)
    assert not fehlend, (
        f"{scheibe} führt die Trigger von {gruppe} ({', '.join(trigger)}), aber nicht "
        f"deren Ergebnisfeld(er): {', '.join(fehlend)}. `_mit_ring_werten()` rechnet sie "
        f"aus, `_scheibe_bindung()` filtert sie dann aus der Deklaration — der Ring rechnet "
        f"ins Leere. Entweder das Feld in die Scheibe aufnehmen oder die Gruppe hier als "
        f"Ausnahme mit Begründung eintragen."
    )


def test_ausnahmeliste_hat_laenge_und_begruendung():
    """Ratsche: die Liste ist leer und bleibt es, solange niemand bewusst parkt.

    Der Längen-Assert ist der Punkt — ohne ihn wächst die Liste unbemerkt und das Gate
    prüft am Ende nichts mehr.
    """
    assert len(RING_LUECKEN_AUSNAHMEN) == 0, (
        f"{len(RING_LUECKEN_AUSNAHMEN)} geparkte Lücke(n): "
        f"{sorted(RING_LUECKEN_AUSNAHMEN)} — jede braucht eine Begründung im Code und "
        f"eine Meldung an den Instructor."
    )
    for schluessel, begruendung in RING_LUECKEN_AUSNAHMEN.items():
        assert len(begruendung) > 20, (
            f"{schluessel}: Begründung zu dünn ({begruendung!r}) — sie muss tragen, "
            f"wenn jemand in sechs Wochen fragt, warum das geparkt ist."
        )


def test_ausnahmeliste_zeigt_auf_echte_faelle():
    """Jede Ausnahme muss ein Paar sein, das es wirklich gibt UND das wirklich lücke ist."""
    vorhanden = {(s, g) for s, g, _t, _e in _gruppen_faelle()}
    for schluessel in RING_LUECKEN_AUSNAHMEN:
        assert schluessel in vorhanden, (
            f"Ausnahme {schluessel} zeigt auf kein anwendbares Paar mehr — streichen."
        )
        scheibe, gruppe = schluessel
        ergebnis = next(e for n, _t, e in RING_GRUPPEN if n == gruppe)
        felder = set(AC.SCHEIBEN[scheibe]["felder"])
        assert set(ergebnis) - felder, (
            f"Ausnahme {schluessel} ist inzwischen repariert — aus RING_LUECKEN_AUSNAHMEN "
            f"streichen, sonst prüft die Liste nichts."
        )


def test_zehn_gruppen_vollstaendig_erfasst():
    """Der Nenner selbst ist geprüft: genau zehn Gruppen, und keine doppelt.

    Ohne diesen Test wäre ein Tippfehler in RING_GRUPPEN unsichtbar — das Gate prüfte dann
    neun Gruppen und meldete grün.
    """
    assert len(RING_GRUPPEN) == 10, (
        f"{len(RING_GRUPPEN)} Gruppen statt zehn — die Liste muss die zehn Gruppen aus "
        f"_mit_ring_werten abbilden."
    )
    namen = [n for n, _t, _e in RING_GRUPPEN]
    assert len(set(namen)) == len(namen), f"doppelte Gruppennamen: {namen}"
    for name, trigger, ergebnis in RING_GRUPPEN:
        assert trigger, f"{name}: ohne Trigger ist die Gruppe nicht prüfbar"
        assert ergebnis, f"{name}: ohne Ergebnisfeld prüft die Gruppe nichts"


def test_trigger_felder_existieren_in_der_bindung():
    """Jedes genannte Feld muss es überhaupt geben — sonst prüft das Gate gegen Luft."""
    sys.path.insert(0, os.path.join(ROOT, "produkt", "traverser"))
    import traverser as TR   # noqa: E402
    bindung = TR.lade_bindung()
    unbekannt = sorted({f for _n, t, e in RING_GRUPPEN for f in t + e} - set(bindung))
    assert not unbekannt, (
        f"Felder in RING_GRUPPEN, die die Bindung nicht kennt: {unbekannt} — "
        f"Tippfehler oder veralteter Name."
    )
