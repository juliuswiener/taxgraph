"""K2-Guard-Fix: Screening-Flag „nie beantwortet" muss denselben Widerspruch sperren wie
„bestätigt-true" (produkt/konsistenz/flag_check.py::flag_widersprueche).

Der Zustandsraum eines Abwesenheits-Flags (kein_vuv, kein_p23_verkauf, ...) hat DREI Werte:
bestätigt-true (Abwesenheit behauptet), bestätigt-false (Nutzer HAT die Einkunftsart), und
unbeantwortet — das Flag steht gar nicht im Snapshot, weil nie ein Event dafür gepostet wurde.
_bestaetigt_wert() (produkt/konsistenz/_helpers.py) liefert für BEIDE „unbeantwortet" UND
„vorhanden, aber nur vorläufig" identisch None — der Guard prüfte vor diesem Fix nur `is not
True`, was beide Fälle gleich behandelte und damit auch „unbeantwortet" als unverdächtig durchließ.

Erreichbarkeit von „ganz abwesend" (nicht nur „vorläufig"): api.py::event() (der einzige
Schreib-Endpunkt) erzwingt keine Dialog-Reihenfolge — ein Betragsfeld kann gepostet werden, ohne
dass das zugehörige Screening-Flag je ein Event bekommen hat. Konkreter Pfad: ein Beleg-/
Kontoauszug-Import (schreiber-Präfix "import:beleg"/"import:kontoauszug", s. api.py Zeile ~515)
kann vv_einnahmen oder p23_veraeusserungspreis direkt aus einem Kontoauszug heraus posten, ohne
dass die UI die Screening-Frage je gestellt hat. store.py::materialisiere() baut den Snapshot
ausschließlich aus dem Event-Log — ein nie gepostetes Feld ist dort schlicht kein Key.

Kein Sackgassen-Guard: die Sperre lässt sich vom Nutzer auflösen. traverser.py::naechste_fragen()
baut die Interview-Queue aus askable Feldern, für die _unbeantwortet(aktiv.get(fid)) gilt (Zeile
214-215: ev is None oder zustand=="vorlaeufig") — das prüft NUR das Event des Flags selbst, nie den
Zustand des Betragsfelds. kein_vuv (bindung_an_gesamt.yaml:289-304) und kein_p23_verkauf
(bindung_an_gesamt.yaml:335-349) sind beide askable:true ohne feld_bedingung/geltungsbedingung, die
sie an ihr jeweiliges Betragsfeld koppelt. Das Kreuz bleibt also in der Fragen-Queue, bis es
beantwortet ist — unabhängig davon, ob der Betrag schon vorliegt.

WAS DIESER FIX NICHT BEHAUPTET: dass die Beobachtungsliste FLAG_NEGIERT für jedes Flag
vollständig ist (welche Feldnamen dort stehen, ist ein UNABHÄNGIGER Fehlertyp — „ein Feld fehlt
in der Liste" statt „ein Zustand fehlt in der Prüfung"). Dieser Fix behandelt ausschließlich den
fehlenden Zustand "unbeantwortet"; die Vollständigkeit der Beobachtungsliste selbst ist hier NICHT
mit erledigt.
"""
from __future__ import annotations

import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for _sub in ("produkt/konsistenz", "produkt/bescheid"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)
import flag_check as FC   # noqa: E402
import bescheid_deklaration as BD   # noqa: E402


def _snap(**felder):
    """{feld_id: (wert, zustand)} -> Snapshot-felder-Ebene."""
    return {fid: {"wert": w, "zustand": z} for fid, (w, z) in felder.items()}


# ---- (b) Normalfall muss weiter "ja" antworten -------------------------------

def test_kontrollzeile_kein_widerspruch_ohne_betrag():
    """Kontrollzeile (a): kein_vuv ganz abwesend, KEIN widersprechender Betrag -> [] unverändert,
    vor und nach dem Fix identisch. Ohne diese Zeile würde der Fix unten nichts beweisen."""
    assert FC.flag_widersprueche(_snap()) == []


def test_normalfall_echte_vuv_bleibt_unversperrt():
    """(b) Normalfall: kein_vuv korrekt bestätigt-false (Nutzer HAT V+V) + Betrag bestätigt ->
    weiterhin KEIN Widerspruch. Ein Fix, der auch den ehrlichen Nutzer sperrt, ist schlimmer als
    der Defekt."""
    assert FC.flag_widersprueche(_snap(kein_vuv=(False, "bestaetigt"),
                                       vv_einnahmen=(1200000, "bestaetigt"))) == []


def test_kontrollzeile_vorlaeufig_bleibt_gruen():
    """Kontrollzeile: das Flag ist DA, nur noch "vorläufig" (Nutzer mitten im Dialog) -> KEIN
    Widerspruch. Dieser Test war schon vor dem Fix grün (test_flag_konsistenz.py::
    test_flag_nur_vorlaeufig_keine_behauptung) und MUSS es bleiben -- "vorläufig" ist nicht
    dasselbe wie "nie gefragt", auch wenn beide _bestaetigt_wert()==None liefern."""
    assert FC.flag_widersprueche(_snap(kein_vuv=(True, "vorlaeufig"),
                                       vv_einnahmen=(1200000, "bestaetigt"))) == []


# ---- Verdachtsfall: unbeantwortet + Betrag -----------------------------------

def test_kein_vuv_unbeantwortet_mit_betrag_sperrt():
    """kein_vuv fehlt GANZ im Snapshot (nie beantwortet), vv_einnahmen bestätigt>0 -> muss
    denselben Widerspruch liefern wie bestätigt-true."""
    w = FC.flag_widersprueche(_snap(vv_einnahmen=(2000000, "bestaetigt")))
    assert len(w) == 1 and w[0]["flag"] == "kein_vuv" and w[0]["feld_id"] == "vv_einnahmen"


# ---- (c) Beobachtungsliste nennt beide Feldnamen wirklich --------------------

def test_beobachtungsliste_kennt_vv_einnahmen():
    """kein_vuv muss vv_einnahmen tatsächlich in seiner FLAG_NEGIERT-Liste haben -- ein Kreuz kann
    alle drei Zustände korrekt behandeln und trotzdem durchlassen, wenn das Feld gar nicht erst in
    der Liste steht."""
    assert "vv_einnahmen" in FC.FLAG_NEGIERT.get("kein_vuv", [])


# ---- p23-Luecke: fremd_arten (bestaetigt-false) und flag_widersprueche (unbeantwortet + ---------
# ---- bestaetigt-true) decken zusammen alle drei Zustaende ab --------------------------------
#
# kein_p23_verkauf steht seit 2026-08-31 in FLAG_NEGIERT (main hat die gegenteilige Entscheidung
# vom selben Tag zurueckgenommen, nachdem eine Messung zeigte: ein unbeantwortetes oder
# faelschlich bestaetigt-true gesetztes Kreuz liess einen echten §23-Gewinn durchrechnen, 3.265.600 ct
# bei grund:"bestaetigt" -- der Betrag verschwindet dabei NICHT, er wird korrekt besteuert, aber
# NICHT erklaert: est_mapping.py (P23_GEWINN) vergibt Kz E0306801 nur, wenn p23_veraeusserungs_typ
# bestaetigt ist, bleibt das Kreuz unbeantwortet bleibt auch die Art unbestaetigt und der Kz-Zweig
# leer, waehrend dieselbe Zahl in der Endsumme steckt. _an_gesamt_sperrgrund()
# (bescheid_deklaration.py Zeile 834) ruft FC.flag_widersprueche(felder, bindung) selbst auf und
# liefert "flag_konsistenz_offen", sobald diese Liste nicht leer ist -- das war schon VOR diesem
# Fix so verdrahtet, nur leer, weil kein_p23_verkauf fehlte. Der fremd_arten-Zweig (nur
# bestaetigt-FALSE, felder.get(fl, {}).get("wert") is False) bleibt DANEBEN der einzige Waechter
# fuer den Fall "Nutzer sagt wahrheitsgemaess, er hat verkauft" (Case 2, Grund noch nicht
# §23-spezifisch, gehoert Julius) -- unbeantwortet UND bestaetigt-true laufen jetzt beide ueber
# denselben flag_widersprueche()-Aufruf in _an_gesamt_sperrgrund() selbst.

def test_p23_unbeantwortet_mit_betrag_flag_check_sperrt():
    """kein_p23_verkauf fehlt GANZ im Snapshot (nie beantwortet), p23-Betrag bestaetigt>0 ->
    _an_gesamt_sperrgrund() sperrt jetzt ueber ihren eigenen flag_widersprueche()-Aufruf
    (Zeile 834) mit grund="flag_konsistenz_offen" (kein_p23_verkauf in FLAG_NEGIERT seit
    2026-08-31). Zieht den vormaligen Doppel-Durchlass-Test um: die Luecke ist zu, hier steht
    jetzt die Zusicherung."""
    felder = _snap(p23_veraeusserungspreis=(2000000, "bestaetigt"))

    grund = BD._an_gesamt_sperrgrund(felder, cfg={"fremd_arten": ("kein_p23_verkauf",),
                                                   "gesamt_guard": True})
    assert grund == "flag_konsistenz_offen", (
        f"_an_gesamt_sperrgrund liefert '{grund}' fuer ein unbeantwortetes kein_p23_verkauf -- "
        f"erwartet 'flag_konsistenz_offen' ueber den flag_widersprueche()-Aufruf in Zeile 834, "
        f"Kommentar oben veraltet")

    w = FC.flag_widersprueche(felder)
    assert len(w) == 1 and w[0]["flag"] == "kein_p23_verkauf", (
        f"flag_widersprueche liefert {w} -- kein_p23_verkauf nicht mehr in FLAG_NEGIERT? "
        f"Kommentar oben veraltet")
