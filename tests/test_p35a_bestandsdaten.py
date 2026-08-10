"""§ 35a nach der Umstellung auf Einzelaufstellung: was passiert mit ALTEN Flat-Werten?

Am 2026-08-10 wurden die drei § 35a-Summenfelder (hh_minijob_aufwendungen, hh_dienstleistungen,
hh_handwerker_arbeitskosten) von askable:true auf askable:false umgestellt. Der Wert kommt
seither aus der Instanzen-Summe (Art + Betrag je Posten), weil checkESt die Einzelaufstellung
verlangt und die blosse Summe mit rc=610001002 ablehnt.

DIESE DATEI DECKT DEN UEBERGANG AB, nicht den Normalfall (der steht in
test_checkest_feldmatrix.py scharf gegen checkESt).

Warum es ihn braucht: `askable` steuert nur das Dialog-Layer (api_llm, traverser, fragekatalog)
und blockiert KEINEN direkten /event-Write — gemessen, ein Flat-Event auf ein askable:false-Feld
wird weiterhin angenommen. Ein Fall, der vor der Umstellung eine Summe bestaetigt hat, traegt
sie also weiter im Store, und es war offen, was daraus wird.

GEMESSEN, und hier festgeschrieben:

    nur alter Flat-Wert 3.000, keine Instanz   rc=610001002   Sum=3000 im XML, Einz fehlt
    Instanz (Betrag + Art)                     rc=0           Sum=3000, Einz da

Der Bestandsfall ist damit **nicht schlechter als vor dem Fix**: der Betrag bleibt erhalten (kein
stiller Verlust), die Erklaerung ist uneinreichbar wie vorher auch, und ERiC benennt woertlich,
was fehlt. Der Nutzer muss seine Angabe einmal als Einzelposten nachtragen — das ist der Preis
der Umstellung, aber er ist sichtbar statt still.

Der gefaehrliche Ausgang waere gewesen, dass die Instanzen-Summe (= 0, es gibt keine Instanz) den
Bestandswert ueberschreibt: dann waere der Abzug ohne Meldung verschwunden. Genau das Muster, das
am selben Tag dreimal Geld gekostet hat (E0205508, KAP-Antragsgrund, Partner-Pauschbetrag).

NEBENBEFUND (gefunden bei diesem Commit, GLEICH mitbehoben statt nur notiert): der obige Test prueft
nur die Deklarationsseite (_mit_ring_werten liest hier den Flat-Wert weiter, injiziert also nichts
Neues drueber). Der RING (_shared_steuer_sonder_agb::_hh_summe, api.py:363) rechnete fuer denselben
Bestandsfall bislang 0 -- reine Instanzen-Summe, kein Fallback. Deklaration und Ring liefen damit
auseinander: XML zeigt 3.000, /ergebnis (VOR jeder Einreichung) haette 0 gerechnet, lautlos. Praktisch
gedeckt, weil so ein Fall wegen der fehlenden Einzelaufstellung ohnehin nicht einreichbar ist -- aber
/ergebnis ist NICHT blockiert und haette den falschen (niedrigeren) Betrag gezeigt. _hh_summe hat
jetzt denselben Flat-Fallback wie die Deklaration; _hh_instanz_positiv (der Abs.5/Abs.3-Guard in
_an_gesamt_sperrgrund) ebenso, sonst haette der Ring den Flat-Betrag zwar mitgerechnet, aber die
CONDITIONAL-MANDATORY-Nachfrage (rechnung_unbar/keine_foerderung) fuer einen Bestandsfall NIE
ausgeloest. test_ring_rechnet_bestandswert_mit und test_guard_greift_auch_fuer_bestandswert_ohne_instanz
unten messen beides.
"""

from __future__ import annotations

import os
import re
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("elster", "produkt/haut", "produkt/import", "produkt/mapping",
            "produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, sub))
sys.path.insert(0, HERE)

import api as API                   # noqa: E402
import checkest_gate as CE          # noqa: E402
import elster_xml as EX             # noqa: E402
import est_mapping                  # noqa: E402
import store as ST                  # noqa: E402

from test_checkest_durchstich import (  # noqa: E402
    _ABSENDER, _HID, _b, _fall_einzel, braucht_eric,
)

FLAT = "hh_handwerker_arbeitskosten"   # altes Summenfeld, seit 2026-08-10 askable:false
BETRAG = "hh_handwerker_betrag"        # neues Instanz-Betragsfeld (Instanz 1 = ohne Suffix)
ART = "hh_handwerker_art"
SUM_KZ = "E0111215"                    # Anlage Haushaltsnahe Zeile 9 (Summe der Lohnanteile)
EINZ_KZ = "E0111214"                   # Zeilen 6-8, darin enthaltene Lohnanteile je Posten


def _xml_und_rc(paare):
    store = _fall_einzel()
    for feld, wert in paare:
        _b(store, feld, wert)
    store = dict(store)
    store["scheibe"] = "gesamt"
    bindung = API._scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)
    felder = API._mit_ring_werten(felder, 2025)
    xml = EX.erzeuge_xml(est_mapping.deklariere(felder, bindung, snapshot_id=sid),
                         vz=2025, hersteller_id=_HID, abgabefaehig=True, **_ABSENDER)
    rc, antwort = CE.validate(xml, "ESt_2025")
    texte = [" ".join(t.split())
             for t in re.findall(r"<Text>(.*?)</Text>", antwort or "", re.S)]
    return xml, rc, texte


def test_askable_false_blockiert_keinen_direkten_write():
    """Vorbedingung der ganzen Datei: askable:false ist KEIN Schreibschutz.

    Braucht kein ERiC. Faellt dieser Test eines Tages, weil echtes Enforcement dazukam, ist der
    Uebergangsfall strukturell unmoeglich geworden und die Datei kann weg — aber das muss
    auffallen und nicht stillschweigend passieren.
    """
    store = _fall_einzel()
    _b(store, FLAT, 300000)
    felder, _ = ST.materialisiere(store)
    assert (felder.get(FLAT) or {}).get("wert") == 300000, (
        "Flat-Event auf ein askable:false-Feld wurde abgelehnt — dann ist askable inzwischen "
        "ein Schreibschutz und diese Datei ist gegenstandslos (siehe Docstring).")


@braucht_eric
def test_bestandswert_bleibt_erhalten_und_der_fall_meldet_was_fehlt():
    """Alter Flat-Wert ohne Instanz: Betrag ueberlebt, Fall sperrt mit sprechender Meldung.

    Zwei Dinge zugleich — beide muessen gelten:
      (a) der Betrag steht weiter im XML (kein stiller Verlust),
      (b) der Fall ist nicht einreichbar, und ERiC sagt woertlich warum.
    Waere nur (a) erfuellt, haetten wir eine Erklaerung, die durchgeht und die Einzelaufstellung
    verschweigt. Waere nur (b) erfuellt, haette der Nutzer seinen Abzug verloren.
    """
    xml, rc, texte = _xml_und_rc([(FLAT, 300000)])

    treffer = re.search(SUM_KZ + r'[^>]*>([^<]*)<', xml)
    assert treffer and treffer.group(1) == "3000", (
        f"Der Bestandswert steht nicht mehr im XML ({treffer.group(1) if treffer else 'fehlt'}) — "
        f"die Instanzen-Summe hat ihn vermutlich auf 0 ueberschrieben. Das waere ein stiller "
        f"Verlust fuer jeden Fall, der vor dem 2026-08-10 eine § 35a-Summe bestaetigt hat.")
    assert EINZ_KZ not in xml, (
        "Ohne Instanz darf kein Einz-Kz im XML stehen — sonst erfaenden wir eine "
        "Einzelaufstellung, die der Nutzer nie gemacht hat.")
    assert rc != CE.RC_OK, (
        "Ein Bestandsfall ohne Einzelaufstellung darf NICHT als einreichbar gelten — checkESt "
        "verlangt sie, und ein gruenes rc hier waere ein falsches Versprechen.")
    assert any("Einzelaufstellung" in t for t in texte), (
        f"Erwartet die Einzelaufstellungs-Beanstandung, damit der Nutzer weiss was zu tun ist. "
        f"Bekommen: {texte[:2]}")


@braucht_eric
def test_instanz_macht_den_fall_einreichbar():
    """Gegenprobe: mit Einzelposten (Betrag + Art) ist derselbe Betrag einreichbar.

    Ohne diese Haelfte wuerde der Test oben auch dann gruen bleiben, wenn § 35a generell
    kaputt waere — dann sperrte jeder Fall, und "sperrt" saehe wie Erfolg aus.
    """
    xml, rc, texte = _xml_und_rc([(BETRAG, 300000), (ART, "Malerarbeiten")])
    assert rc == CE.RC_OK, (
        f"Mit vollstaendigem Einzelposten muss der Fall einreichbar sein, rc={rc}: {texte[:2]}")
    assert EINZ_KZ in xml and SUM_KZ in xml, (
        f"Beide Kz muessen ins XML: Einz={EINZ_KZ in xml}, Sum={SUM_KZ in xml}")


# Voller Pflicht-Kegel fuer Scheibe "gesamt" (SCHEIBEN["gesamt"]["kegel"], api_constants.py) --
# _fall_einzel() (test_checkest_durchstich.py) deckt ihn NICHT ab, das reicht fuer die checkESt-XML-
# Tests oben (die gehen ueber _mit_ring_werten, kein Kegel-Gate), aber _feste_zahl ist fail-closed und
# verlangt ALLE Kegel-Felder bestaetigt. 1:1 aus GESAMT_KEGEL_BASIS (test_ring_regression_kampagne.py)
# + hh_in_eu_ewr (§ 35a Abs. 4 gatet alle drei Toepfe, sonst bleibt jeder Betrag wirkungslos 0) -- lokal
# kopiert statt importiert, weil jenes Modul beim Import pytest.importorskip("jsonschema") auswertet
# und diese Datei sonst mitspringen wuerde, wenn jsonschema fehlt.
_GESAMT_KEGEL = (
    ("veranlagung", "einzel"),
    ("bruttoarbeitslohn", 20000000),
    ("vv_einnahmen", 0), ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0),
    ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 100),
    ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0),
    ("versicherungsart", "gesetzlich_an"), ("vorsorge_arbeitslosenversicherung", 0),
    ("vorsorge_erwerbsunfaehigkeit", 0), ("vorsorge_unfall_haftpflicht", 0),
    ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0),
    ("mit_anspruch_auf_zuschuss", False),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
    ("hh_in_eu_ewr", True),
)


def _zahl_cent(paare):
    """Rechnet zahl_cent auf Scheibe 'gesamt' -- dieselben API-Bausteine wie ergebnis() (api.py:2374),
    nur ohne den HTTP-/Datei-Store-Umweg. Bricht die Vorbedingungen laut, statt None durchzureichen,
    damit ein spaeterer Guard-Fund hier nicht als "0 == 0" falsch gruen wird."""
    store = ST.leerer_store(2025, fall_id="p35a_gesamt_voll")
    for feld, wert in _GESAMT_KEGEL + tuple(paare):
        _b(store, feld, wert)
    store["scheibe"] = "gesamt"
    cfg = API._cfg(store)
    bindung = API._scheibe_bindung(store)
    felder, _ = ST.materialisiere(store)
    scheibe_felder = cfg.get("kegel") or API._scheibe_felder(store)
    vz = int(store["veranlagungszeitraum"])
    sperr = API._an_gesamt_sperrgrund(felder, cfg, vz, store, bindung) if cfg.get("guard") else None
    assert not sperr, f"Vorbedingung kaputt: Ring gesperrt ({sperr!r}), zahl_cent nicht vergleichbar."
    ergebnis = API._feste_zahl(felder, bindung, cfg, vz, scheibe_felder, store)
    assert ergebnis is not None, "Vorbedingung kaputt: _feste_zahl liefert None."
    return ergebnis[0]


def test_ring_rechnet_bestandswert_mit():
    """Der NEBENBEFUND aus dem Docstring oben, konkret gemessen: Bestandsfall (Flat-Wert,
    keine Instanz) und Instanz-Fall muessen fuer denselben Betrag dasselbe zahl_cent liefern.

    Minijob statt Handwerker, weil § 35a Abs. 1 KEIN CONDITIONAL-MANDATORY-Begleitfeld braucht
    (anders als Abs. 2/3 -- hh_rechnung_unbar). Mit Handwerker waere der Bestandsfall vom Guard
    gesperrt (kein Wert bestaetigt fuer hh_rechnung_unbar), zahl_cent bliebe in beiden Faellen
    None und der Vergleich saehe nichts.
    """
    bestand = _zahl_cent([("hh_minijob_aufwendungen", 100000)])
    instanz = _zahl_cent([("hh_minijob_betrag", 100000), ("hh_minijob_art", "Haushaltshilfe")])
    assert bestand == instanz, (
        f"Bestandsfall (Flat-Wert, {bestand} ct) und Instanz-Fall ({instanz} ct) liefern "
        f"verschiedenes zahl_cent fuer denselben Minijob-Betrag -- der Ring rechnet den "
        f"Bestandswert nicht mit, obwohl das XML ihn (siehe Test oben) weiter deklariert.")


def test_guard_greift_auch_fuer_bestandswert_ohne_instanz():
    """Kehrseite von test_ring_rechnet_bestandswert_mit: wenn der Ring den Flat-Wert jetzt
    mitrechnet, MUSS der Abs.5-Guard ihn auch sehen -- sonst liefe ein Bestandsfall mit
    Handwerker-Flat-Wert ohne Zahlungsart-Bestaetigung durch den Ring, obwohl § 35a Abs. 5 S. 3
    die unbare Zahlung fuer JEDEN Handwerker-Abzug verlangt (nicht nur fuer neu erfasste)."""
    store = _fall_einzel()
    _b(store, FLAT, 300000)   # hh_handwerker_arbeitskosten, Flat-Bestandswert, kein hh_rechnung_unbar
    store["scheibe"] = "gesamt"
    cfg = API._cfg(store)
    bindung = API._scheibe_bindung(store)
    felder, _ = ST.materialisiere(store)
    vz = int(store["veranlagungszeitraum"])
    grund = API._an_gesamt_sperrgrund(felder, cfg, vz, store, bindung)
    assert grund == "rechnung_unbar_offen", (
        f"Erwartet rechnung_unbar_offen fuer einen Bestandsfall mit Handwerker-Flat-Wert ohne "
        f"Zahlungsart-Bestaetigung, bekommen: {grund!r}.")
