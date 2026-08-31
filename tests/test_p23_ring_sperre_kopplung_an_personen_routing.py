"""WAECHTER (main-Auftrag 2026-08-31): Kopplung zwischen zwei Befunden aus zwei verschiedenen
Strängen, die bisher niemand gemeinsam bewacht.

BEFUND 1 (Nachbarinstanz, tests/test_p23_partner_verkauf_still_unter_person_a_eingereicht.py,
HEAD 3bfca6b): §23-Verkaeufe routet erzeuge_xml() rein indexbasiert -- KEIN Personenfeld
entscheidet, wessen Verkauf es ist. Zwei Verkaeufe verschiedener Personen liefern
{(PersonA,45000),(PersonA,8000)} statt {(PersonA,45000),(PersonB,8000)}. Dieser Test
wiederholt jene Zielaussage NICHT (Doppelarbeit, main-Weisung) -- er prueft stattdessen die
URSACHE, eine Ebene tiefer: est_mapping.py schreibt §23-Kennzahlen NIE in `person_b`, den
Mechanismus, der KAP-/N-/G-Partnerfelder korrekt nach PersonB routet (produkt/import/
elster_xml.py, `_person_b_index`/`person_override` -- nur der `person_b`-Dict-Pfad, NIE der
`anlage_instanzen`-Pfad, den §23 tatsaechlich benutzt, uebergibt `person_override`). Es gibt fuer
§23 nicht einmal ein `_partner`-Eingabefeld -- der Nutzer hat strukturell keine Moeglichkeit,
"das war der Verkauf meines Partners" ueberhaupt einzugeben.

BEFUND 2 (eigene Messung, HTTP-Dialogdurchlauf gegen /tmp-Worktree HEAD 714b40e, Scheibe
`gesamt`, GET /fragen + POST /event ausschliesslich in Katalog-Reihenfolge, kein Vorgriff): ein
wahrheitsgemaess bestaetigter §23-Verkauf sperrt seit HEAD 714b40e (Fix fuer den 21.000-EUR-
Fund, tests/test_p23_verkauf_beim_rentner_sperrt_statt_still_falsch.py) mit
grund=einkunftsart_nicht_ring_faehig, BEVOR ueberhaupt XML entsteht -- auf BEIDEN Scheiben
(`gesamt` UND `rentner_gesamt`, api_constants.py: `fremd_arten` fuehrt `kein_p23_verkauf` auf
beiden). 192 gestellte Fragen im vollstaendigen Dialog-Durchlauf auf Scheibe `gesamt` bis die
Warteschlange leer war, alle 6 Partner-Kegel-Felder (GESAMT_PARTNER_19 + GESAMT_PARTNER_KAP)
darunter und beantwortbar -- kein Dialog-Sackgasse, echte, erreichbare Sperre.

DIE KOPPLUNG: solange Befund 1 gilt (Routing indexbasiert, kein Personenfeld), haelt die Sperre
aus Befund 2 den Personen-Fehler vom echten Dialog fern -- er lebt weiter, nur unerreichbar.
Repariert jemand die Befreiungstatbestaende und nimmt `kein_p23_verkauf` aus `fremd_arten`, OHNE
das Routing zu reparieren, wird der Personen-Fehler live: ein Ehepaar reicht den Verkauf des
Partners unter der eigenen Person ein. Diese Reihenfolge (Routing VOR Befreiungsabfrage) ist der
Punkt dieses Wächters.

MUTATIONSPROBE MIT RUECKWEG (main-Auflage: "ohne den Rueckweg ist es kein Beweis", Vorbild
tests/test_screening_partner.py::test_mutationsprobe_der_waechter_wuerde_den_geldfehler_finden):
`_kopplungsverstoss()` ist der wiederverwendbare Kern. Er wird zuerst mit der ECHTEN Konfiguration
(aus api_constants.py importiert, hier NIRGENDS editiert) aufgerufen -- gruen, weil die Sperre
haelt. Dann mit einer LOKALEN, absichtlich entschaerften Kopie (kein_p23_verkauf aus fremd_arten
entfernt, api_constants.py selbst unangetastet) -- rot, mit benanntem Grund. Zuletzt wieder mit
der echten Konfiguration -- wieder gruen (der Rueckweg). api_constants.py und die bindung-YAML
bleiben gesperrt (main-Auflage) -- die Mutation geschieht ausschliesslich an einer lokalen
dict-Kopie, nie am Modul selbst.

NULL LLM.
"""
from __future__ import annotations

import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for sub in ("produkt/import", "produkt/mapping", "produkt/store", "produkt/traverser",
            "produkt/haut"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api_constants as APC    # noqa: E402
import est_mapping             # noqa: E402
import store as ST             # noqa: E402
import traverser as TR         # noqa: E402

TS = "2026-08-31T02:00:00+00:00"
H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}


def _b(s, feld_id, wert):
    ST.append_event(store=s, feld_id=feld_id, wert=wert, zustand="bestaetigt", herkunft=H,
                     schreiber="ui:laie", signal={"signal_1": None, "signal_2": f"ok@{feld_id}"}, ts=TS)


def _fall_zusammen_ein_verkauf(fall_id):
    """Zusammenveranlagung, EIN bestaetigter §23-Verkauf -- welcher der beiden Personen er
    gehoert, ist ueber den Store gar nicht aussagbar (kein `_partner`-Feld existiert fuer p23,
    Befund 1 oben). Genau das misst `_p23_kz_in_person_b()` unten strukturell nach."""
    s = ST.leerer_store(2025, fall_id=fall_id)
    _b(s, "veranlagung", "zusammen")
    _b(s, "bruttoarbeitslohn", 6000000)
    _b(s, "kein_gewinn", True)
    _b(s, "kein_kap", True)
    _b(s, "kein_vuv", True)
    _b(s, "kein_sonstige", False)
    _b(s, "p23_veraeusserungs_typ", "grundstueck")
    _b(s, "p23_veraeusserungspreis", 20000000)               # 200.000 EUR
    _b(s, "p23_anschaffung_herstellungskosten", 15000000)    # 150.000 EUR -> Gewinn 50.000 EUR
    _b(s, "p23_werbungskosten", 0)
    return s


def _p23_kz_in_person_b(bindung) -> set[str]:
    """URSACHE, live gemessen: welche §23-Kennzahlen stehen in `person_b`?

    Leer = das Routing ist weiterhin indexbasiert (est_mapping.py schreibt §23 nur ueber
    `anlage_instanzen`, nie ueber `person_b`) -- die Kopplung unten greift. Nicht-leer wuerde
    heissen: Befund 1 ist repariert, dieser Waechter (und mit ihm die Sperre in fremd_arten)
    waeren nicht mehr noetig.
    """
    snap, sid = ST.materialisiere(_fall_zusammen_ein_verkauf("kopplung_person_b_messung"))
    result = est_mapping.deklariere(snap, bindung, snapshot_id=sid)
    p23_kz = {kz for eintrag in result.get("anlage_instanzen", {}).get("p23_veraeusserung", [])
              for kz in eintrag["felder"]}
    assert p23_kz, (
        "Messaufbau kaputt: kein einziges §23-Kz aus anlage_instanzen['p23_veraeusserung'] -- der "
        "Verkauf im Testfall wurde nicht erfasst, die Ursachen-Messung sagt dann nichts aus.")
    return p23_kz & set(result.get("person_b", {}))


def _kopplungsverstoss(cfg_gesamt: dict, cfg_rentner: dict,
                       p23_kz_in_person_b: set[str]) -> list[str]:
    """Der wiederverwendbare Kern des Waechters (Vorbild: test_screening_partner.py,
    `_partner_felder_an_ich_kreuz`). Gibt die Scheiben zurueck, auf denen die Kopplung verletzt
    ist: Routing weiterhin indexbasiert (kein §23-Kz in person_b), aber die Ring-Sperre
    (`kein_p23_verkauf` in `fremd_arten`) fehlt."""
    if p23_kz_in_person_b:
        return []   # Routing waere repariert -- die Kopplung entfaellt, keine Sperrpflicht mehr
    return [name for name, cfg in (("gesamt", cfg_gesamt), ("rentner_gesamt", cfg_rentner))
            if "kein_p23_verkauf" not in cfg.get("fremd_arten", ())]


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


def test_ring_sperre_haelt_solange_p23_indexbasiert_routet(bindung):
    """Der eigentliche Waechter, gegen die ECHTE Konfiguration aus api_constants.py (nur
    importiert, hier nirgends editiert). Bricht jemand die Kopplung fuer echt -- die Sperre
    lockern, ohne das Routing zu reparieren --, wird DIESER Test rot und nennt die Scheibe(n)."""
    p23_kz_in_person_b = _p23_kz_in_person_b(bindung)
    verstoss = _kopplungsverstoss(APC.SCHEIBEN["gesamt"], APC.SCHEIBEN["rentner_gesamt"],
                                  p23_kz_in_person_b)
    assert not verstoss, (
        f"KOPPLUNG GEBROCHEN auf Scheibe(n) {verstoss}: das Personen-Routing ist weiterhin "
        f"indexbasiert ohne Personenfeld (est_mapping.py verdrahtet §23-Kz nie nach person_b), "
        f"aber die Ring-Sperre 'kein_p23_verkauf' fehlt in fremd_arten -- ein bestaetigter "
        f"Partner-Verkauf wuerde jetzt live unter der falschen Person eingereicht. Repariere "
        f"erst das Personen-Routing (elster_xml.py/est_mapping.py), bevor die Sperre faellt.")


def test_mutationsprobe_kopplung_mit_rueckweg(bindung):
    """Beweist, dass der Waechter oben nicht zufaellig gruen ist: entschaerft `fremd_arten`
    lokal (API.SCHEIBEN/api_constants.py selbst bleibt unangetastet) -> Kopplung muss rot werden
    und die betroffene(n) Scheibe(n) nennen -> Original-Konfiguration wieder eingesetzt ->
    Kopplung wieder gruen. main-Auflage: "ohne den Rueckweg ist es kein Beweis"."""
    p23_kz_in_person_b = _p23_kz_in_person_b(bindung)
    assert not p23_kz_in_person_b, (
        "Dieser Test setzt voraus, dass das Routing (noch) kaputt ist -- ist es repariert, sagt "
        "die Mutationsprobe nichts mehr aus und muss neu ueberlegt werden.")

    echt_gesamt = APC.SCHEIBEN["gesamt"]
    echt_rentner = APC.SCHEIBEN["rentner_gesamt"]

    # 1) VOR der Mutation: echte Konfiguration -- gruen.
    assert _kopplungsverstoss(echt_gesamt, echt_rentner, p23_kz_in_person_b) == [], (
        "Schon vor der Mutationsprobe verletzt die echte Konfiguration die Kopplung -- dann "
        "misst die Probe unten nichts Neues.")

    # 2) MUTATION: lokale Kopie ohne 'kein_p23_verkauf' in fremd_arten -- NUR bei 'gesamt', damit
    #    sich am Ergebnis ablesen laesst, dass die Meldung die richtige Scheibe benennt.
    entschaerft_gesamt = dict(echt_gesamt)
    entschaerft_gesamt["fremd_arten"] = tuple(
        fl for fl in echt_gesamt["fremd_arten"] if fl != "kein_p23_verkauf")
    assert "kein_p23_verkauf" not in entschaerft_gesamt["fremd_arten"], (
        "Die Mutationsprobe baut die Entschaerfung nicht ein -- sie kann die Kopplung dann auch "
        "nicht rot zeigen.")

    verstoss_entschaerft = _kopplungsverstoss(entschaerft_gesamt, echt_rentner, p23_kz_in_person_b)
    assert verstoss_entschaerft == ["gesamt"], (
        f"Die Mutationsprobe haette genau ['gesamt'] als Kopplungsverstoss zeigen muessen (die "
        f"einzige entschaerfte Scheibe), tatsaechlich: {verstoss_entschaerft}. Der Waechter oben "
        f"benennt die falsche(n) Scheibe(n) oder findet die Entschaerfung gar nicht.")

    # 3) RUECKWEG: echte Konfiguration erneut -- wieder gruen. api_constants.py wurde dafuer nie
    #    angefasst, nur die lokale Kopie oben verworfen.
    assert _kopplungsverstoss(echt_gesamt, echt_rentner, p23_kz_in_person_b) == [], (
        "Ruecklauf fehlgeschlagen: mit der unveraenderten echten Konfiguration muss die Kopplung "
        "wieder gruen sein -- sonst haengt der Test an der lokalen Mutation, nicht an einem "
        "sauberen Vergleich.")
