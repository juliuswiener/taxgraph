"""Der Partner wird gefragt wie der Nutzer — nicht mehr und nicht weniger. NULL LLM.

GEMESSEN am 2026-08-27 in einem echten Durchgang: 145 Antworten, davon rund 32 zum Partner und
davon rund 25 mit dem Wert 0 oder False. Zwei Ursachen, beide in der Bindung:

  1. `geburtsjahr` wird aus `stammdaten_geburtsdatum` ABGELEITET und deshalb nie gefragt.
     `geburtsjahr_partner` wurde GEFRAGT, obwohl das Geburtsdatum des Partners unmittelbar davor
     erhoben worden war. Die Ableitung fehlte nur auf der Partner-Seite.

  2. Vier Fragen lesen aus der Lohnsteuerbescheinigung des PARTNERS ab (Steuerklasse, Lohnsteuer,
     Arbeitnehmer- und Arbeitgeberanteil zur Rentenversicherung). Ohne Arbeitslohn gibt es diese
     Bescheinigung nicht. Im Durchgang kam `bruttoarbeitslohn_partner = 0` NACH ihnen.

Der teuerste Test in dieser Datei ist der GEGENPROBEN-Test: eine Bedingung, die im Normalfall
zuschlaegt, nimmt dem Nutzer echte Abzuege weg. Genau das ist hier schon zweimal passiert (der
Verpflegungsmehraufwand ging komplett verloren, einmal verschwanden 351 EUR still). Deshalb wird
hier nicht nur geprueft, dass die Frage bei bestaetigter Null entfaellt, sondern vor allem, dass
sie in JEDEM anderen Fall stehen bleibt — bei Einkommen, bei Schweigen und bei einem bloss
vorlaeufigen Vorschlag.
"""
from __future__ import annotations

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/traverser", "produkt/store"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import store as ST      # noqa: E402
import traverser as TR  # noqa: E402

BINDUNG = TR.lade_bindung()
# Ein Vorschlags-Schreiber (Beleg) braucht den Katalog: er entscheidet, welche Felder ueberhaupt
# vorgeschlagen werden duerfen (Wahlrechte und Abwesenheits-Erklaerungen bleiben beim Menschen).
KATALOG = ST.lade_katalog(BINDUNG)

# Die Felder, die ohne Arbeitslohn des Partners gegenstandslos sind. `kirchensteuer_arbeitgeber_
# partner` gehoert der Sache nach dazu, traegt aber schon eine `feld_bedingung` auf
# `kist_konfession_partner` — das Schema laesst nur EINE je Feld zu (s. Bericht).
LSTB_PARTNER = ("p36_lohnsteuer_partner", "steuerklasse_partner",
                "vor_an_anteil_rv_partner", "vor_ag_anteil_rv_partner")

# Vorsorge-Felder des Partners, die AUCH OHNE Arbeitslohn Betraege tragen: Kranken- und
# Pflegeversicherung zahlt auch ein nicht angestellter Partner, und `vor_rv_ausserhalb_lstb_partner`
# ist ausdruecklich der Weg fuer Beitraege ausserhalb der Lohnsteuerbescheinigung. Waeren sie
# mitgegated, verloere ein Partner ohne Anstellung seinen Vorsorgeabzug.
NICHT_GEGATED = ("basis_kv_partner", "basis_pv_partner", "vor_rv_ausserhalb_lstb_partner",
                 "vorsorge_arbeitslosenversicherung_partner", "vorsorge_erwerbsunfaehigkeit_partner",
                 "vorsorge_unfall_haftpflicht_partner")


def _setze(store, feld, wert, zustand="bestaetigt"):
    # Der Beleg-Weg ist der realistische Fall fuer einen vorlaeufigen Wert: eine Extraktion aus
    # der Lohnsteuerbescheinigung. `beleg_import` + `signal_2: null` ist dort Pflicht (Auflage A
    # in store.append_event) — ein Beleg-Import bestaetigt strukturell nie selbst.
    herkunft = ({"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}
                if zustand == "bestaetigt"
                else {"herkunft": "beleg_import", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"})
    ST.append_event(store, feld_id=feld, wert=wert, zustand=zustand, herkunft=herkunft,
                    schreiber="ui:laie" if zustand == "bestaetigt" else "import:beleg",
                    signal={"signal_1": None,
                            "signal_2": f"klick@{feld}" if zustand == "bestaetigt" else None},
                    ts="2026-08-27T12:00:00+00:00", bindung=BINDUNG,
                    katalog=KATALOG)


def _queue(*paare):
    """Die echte Interview-Queue nach den angegebenen Antworten."""
    s = ST.leerer_store(2025, fall_id="partner-gleich")
    for feld, wert, *rest in paare:
        _setze(s, feld, wert, rest[0] if rest else "bestaetigt")
    return TR.naechste_fragen(s, BINDUNG)


def _felder(store_paare):
    return set(_queue(*store_paare))


# ---- 1) Die fehlende Ableitung -----------------------------------------------

def test_geburtsjahr_des_partners_wird_abgeleitet_nicht_gefragt():
    """Der gemessene Fall: das Geburtsdatum des Partners liegt vor, die Jahresfrage kam trotzdem."""
    q = _queue(("veranlagung", "zusammen"), ("stammdaten_geburtsdatum_partner", "07.07.1982"))
    assert "geburtsjahr_partner" not in q


def test_abgeleitetes_geburtsjahr_des_partners_ist_richtig():
    """Nicht nur „entfaellt", sondern der richtige Wert — sonst waere die Frage gegen einen
    falschen Kohorten-Schluessel (§ 24a S. 5 EStG) eingetauscht."""
    s = ST.leerer_store(2025, fall_id="partner-gj")
    _setze(s, "veranlagung", "zusammen")
    _setze(s, "stammdaten_geburtsdatum_partner", "07.07.1982")
    felder, _ = ST.materialisiere(s)
    assert felder["geburtsjahr_partner"]["wert"] == 1982
    assert felder["geburtsjahr_partner"]["zustand"] == "bestaetigt"


def test_geburtsjahr_partner_bleibt_stehen_ohne_geburtsdatum():
    """Der Negativfall: ohne Quellangabe darf die Ableitung die Frage NICHT wegnehmen.

    Die vorlaeufige Variante ist fuer dieses Feld nicht pruefbar und muss es auch nicht sein:
    `stammdaten_geburtsdatum_partner` steht in keinem Vorschlags-Katalog (human-only), ein
    vorlaeufiger Wert kann dort also gar nicht erst entstehen — store.append_event weist ihn
    fail-closed ab. Fuer die Torfrage, wo der Beleg-Weg offensteht, deckt
    test_vorlaeufige_null_schliesst_nicht_aus denselben Punkt ab."""
    q = _queue(("veranlagung", "zusammen"))
    assert "geburtsjahr_partner" in q


def test_beide_seiten_leiten_gleich_ab():
    """Die Asymmetrie selbst, als Gate: dasselbe Geburtsdatum-Muster fuer beide Personen."""
    q = _queue(("veranlagung", "zusammen"),
               ("stammdaten_geburtsdatum", "05.05.1980"),
               ("stammdaten_geburtsdatum_partner", "07.07.1982"))
    assert "geburtsjahr" not in q and "geburtsjahr_partner" not in q


def test_keine_ableitung_ist_nur_auf_einer_seite_deklariert():
    """Systematischer Waechter statt Einzelfall: kein Nutzer-Feld darf eine Ableitung tragen,
    deren Partner-Gegenstueck sie nicht hat. Genau diese Luecke war der Befund."""
    einseitig = []
    for fid, b in BINDUNG.items():
        if not fid.endswith("_partner"):
            continue
        basis = BINDUNG.get(fid[: -len("_partner")])
        if basis and basis.get("ableitung") and not b.get("ableitung"):
            einseitig.append(fid)
    assert not einseitig, (
        f"Nutzer-Feld abgeleitet, Partner-Feld gefragt: {einseitig}. Traegt das Quellfeld der "
        f"Ableitung ein `_partner`-Gegenstueck, gehoert dieselbe Ableitung dorthin.")


# ---- 2) Die fehlende Torfrage ------------------------------------------------

def test_ohne_arbeitslohn_entfallen_die_bescheinigungsfragen():
    """Der gemessene Fall: Partner ohne Arbeitslohn, trotzdem vier Fragen aus seiner
    Lohnsteuerbescheinigung."""
    q = _queue(("veranlagung", "zusammen"), ("bruttoarbeitslohn_partner", 0))
    gestellt = [f for f in LSTB_PARTNER if f in q]
    assert not gestellt, f"trotz bestaetigter Null noch gefragt: {gestellt}"


def test_mit_arbeitslohn_werden_sie_alle_gefragt():
    """DIE GEGENPROBE. Der Normalfall — Partner mit Einkommen — muss jede dieser Fragen
    weiterhin bekommen. Faellt dieser Test, ist ein Abzug abgeschaltet."""
    q = _queue(("veranlagung", "zusammen"), ("bruttoarbeitslohn_partner", 4_000_000))
    fehlend = [f for f in LSTB_PARTNER if f not in q]
    assert not fehlend, f"Partner MIT Arbeitslohn bekommt diese Fragen nicht mehr: {fehlend}"


def test_unbeantwortet_heisst_fragen():
    """Fail-closed, die Kernregel: Schweigen schliesst nichts aus. Wer die Torfrage ueberspringt,
    bekommt die Folgefragen — lieber eine Frage zu viel als ein verlorener Abzug."""
    q = _queue(("veranlagung", "zusammen"))
    fehlend = [f for f in LSTB_PARTNER if f not in q]
    assert not fehlend, f"unbeantwortete Torfrage hat Fragen weggenommen: {fehlend}"


def test_vorlaeufige_null_schliesst_nicht_aus():
    """Ein Beleg-Vorschlag mit 0 ist keine Antwort des Nutzers. Wuerde er ausschliessen, naehme
    eine Belegextraktion dem Nutzer vier Fragen weg, die er nie gesehen hat."""
    q = _queue(("veranlagung", "zusammen"), ("bruttoarbeitslohn_partner", 0, "vorlaeufig"))
    fehlend = [f for f in LSTB_PARTNER if f not in q]
    assert not fehlend, f"vorlaeufige Null hat Fragen weggenommen: {fehlend}"


def test_vorsorgefelder_des_partners_bleiben_auch_ohne_arbeitslohn():
    """DER GELDTEST. Kranken- und Pflegeversicherung zahlt auch ein Partner ohne Anstellung, und
    Rentenbeitraege ausserhalb der Lohnsteuerbescheinigung sind genau der Weg fuer einen
    freiwillig oder als Selbstaendiger Einzahlenden. Waeren diese Felder mitgegated, verloere er
    seinen Vorsorgeabzug (§ 10 Abs. 1 Nr. 2 und 3 EStG) — die teuerste Art, eine Frage zu sparen."""
    q = _queue(("veranlagung", "zusammen"), ("bruttoarbeitslohn_partner", 0))
    fehlend = [f for f in NICHT_GEGATED if f not in q]
    assert not fehlend, (
        f"Vorsorge-Felder des Partners entfallen bei Arbeitslohn 0: {fehlend}. Sie haengen NICHT "
        f"am Arbeitslohn — hier verschwindet ein Abzug.")


def test_torfrage_steht_vor_den_fragen_die_sie_erspart():
    """Die Naht zwischen Deklaration und Wirkung. Ohne `eingangsfrage: true` stand die Torfrage
    auf Platz 112, `p36_lohnsteuer_partner` auf 99 und `steuerklasse_partner` auf 108 — die
    Bedingung waere fuer sie nie zum Zuge gekommen, weil beide eine `geltungsbedingung` tragen
    und deshalb vor den Betragsfeldern sortieren."""
    q = _queue(("veranlagung", "zusammen"))
    platz = {f: i for i, f in enumerate(q)}
    assert "bruttoarbeitslohn_partner" in platz, "Torfrage steht gar nicht in der Queue"
    davor = [f for f in LSTB_PARTNER
             if f in platz and platz[f] < platz["bruttoarbeitslohn_partner"]]
    assert not davor, (
        f"{davor} kommen VOR der Torfrage (Platz {platz['bruttoarbeitslohn_partner']}) — "
        f"fuer sie wirkt die Bedingung nicht.")


def test_die_bedingung_zeigt_auf_die_torfrage():
    """Deklaration statt Zufall: alle vier haengen am selben Feld, und zwar ueber `wert_nicht`.
    Mit `wert: 0` stuende die Polaritaet auf dem Kopf — gefragt wuerde dann NUR bei Null."""
    for fid in LSTB_PARTNER:
        bed = BINDUNG[fid].get("feld_bedingung")
        assert bed, f"{fid} hat keine feld_bedingung"
        assert bed["feld"] == "bruttoarbeitslohn_partner", f"{fid} haengt an {bed['feld']}"
        assert bed.get("wert_nicht") == 0, (
            f"{fid} nutzt nicht `wert_nicht: 0` — mit `wert` waere die Polaritaet umgekehrt und "
            f"die Fragen kaemen genau dann, wenn der Partner NICHTS verdient hat.")
