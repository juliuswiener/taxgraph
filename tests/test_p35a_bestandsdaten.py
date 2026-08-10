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
