"""A_B_LP-Summenfehler (§ 10 Abs. 1 Nr. 3a EStG, "sonstige Vorsorgeaufwendungen").

XSD-Befund (E10-2025.xsd, ~/02_Software/eric/doc_extract/ERiC-44.2.4.0/.../ESt/Schema/2025/,
Zeilen 24805-24978, direkt am ERiC-Schema gelesen, nicht am Repo): der Container
`Weit_Sons_VorAW_..._CType` hat zwei Geschwister-Elemente -- `Pers` (maxOccurs=2, mit
PFLICHT-Diskriminator `Person` A/B) und `A_B_LP` (maxOccurs=1, KEIN Diskriminator, auch nicht
auf der `Einz`-Ebene darunter). `A_B_LP` enthaelt 5 Kategorien (AL_Vers, ErwU_BU_Vers,
U_HP_Ris_Vers, RV_m_WR_KapLV, RV_o_WR_o_AV), jede mit genau einem `Sum`-Kz, dessen
`<xs:documentation>` woertlich "Summe" lautet:
    E2001403 (AL_Vers)        E2001503 (ErwU_BU_Vers)     E2001803 (U_HP_Ris_Vers)
    E2001903 (RV_m_WR_KapLV)  E2002003 (RV_o_WR_o_AV)
Der fehlende Diskriminator direkt neben einem belegten personenbezogenen Geschwister-Pfad
macht "gemeinsame Summe beider Ehegatten" zu einer Schema-Aussage, nicht zu einer aus dem
Namen erschlossenen.

Live gemessen (2026-08-30, /tmp/messe_kostet_ausgang2_geld.py, SCHRITT 3): bei Zusammen-
veranlagung mit Beitraegen auf BEIDEN Seiten traegt jedes der 5 Sum-Kz im erzeugten XML nur
den Betrag von Person A. Der Partner-Betrag verschwindet spurlos -- kein Sperrgrund, kein
Hinweis, `eingaben_konsistent` bleibt True.

Reparaturort: produkt/mapping/est_mapping.py:654-655 (generische Klasse-1/b-Route,
`elif b.get("elster_kz"): deklaration[b["elster_kz"]] = ...` -- reines Ueberschreiben, nie
Addition). Ein Fix muesste fuer die 5 A_B_LP-Sum-Kz Person-A- und Partner-Beitrag addieren,
statt den zweiten Aufruf den ersten ueberschreiben zu lassen.

Miss am ECHTEN erzeugten XML (ET.fromstring + Kz-Text), nicht am flachen `deklaration`-Dict --
die Naht sitzt zwischen den beiden Repraesentationen, ein Blick nur auf `deklaration` haette
den Fehler frueher schon verdeckt (§23-Nebenbefund vom selben Tag).

Zusatzbefund (2026-08-30, NICHT die Grundlage der Erwartung unten): alle 5 `elster_kz_grund`-
Texte in produkt/bindung/bindung_an_gesamt.yaml behaupten woertlich, der `Pers`-Zweig fuehre
"E2004403" -- korrekt fuer AL_Vers (dort real additiv, wie eigens im XSD gemessen), aber
copy-paste-falsch fuer die anderen 4 Kategorien, die mit E2004403 nichts zu tun haben. Und
E2004403 kommt in est_mapping.py kein einziges Mal vor -- ein Zitat auf ein real existierendes,
aber falsch zugeordnetes Kz, gefaehrlicher als ein erfundenes, weil es beim Gegen-Grep im Schema
einen Treffer liefert und sich damit scheinbar selbst bestaetigt. Die Erwartung unten steht
ausschliesslich auf dem XSD-Befund oben (A_B_LP, maxOccurs=1, kein Diskriminator, documentation=
"Summe"), nie auf diesem Text.

Drei Teile:
  test_person_a_allein_traegt_ihren_betrag  -- GRUENE KONTROLLE. Nur Person A hat Beitraege ->
      Sum-Kz = A's Betrag. Muss nach einem Fix fuer den xfail unten weiterhin gruen bleiben,
      sonst sperrt der Fix den heute schon korrekten Normalfall.
  test_person_a_und_partner_werden_addiert  -- xfail(strict=True), der Defekt. A und Partner
      mit UNTERSCHEIDBAREN Betraegen (nicht gleich -- sonst saehe A's Alleinwert wie eine
      funktionierende Summe aus). Erwartung: Sum-Kz = A + Partner. Heute: nur A.
Beide ueber alle 5 Kategorien parametrisiert (mind. 2 gefordert; Einheitlichkeit war bisher
nur gemessen, nicht durch einen Test geprueft -- dieser Test prueft sie fuer jede Kategorie
einzeln, statt sie von einer stellvertretend zu uebernehmen).

NULL LLM.
"""
from __future__ import annotations

import os
import sys
import xml.etree.ElementTree as ET

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for _sub in ("produkt/import", "produkt/mapping", "produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import elster_xml as EX      # noqa: E402
import est_mapping           # noqa: E402
import store as ST           # noqa: E402
import traverser as TR       # noqa: E402

from test_checkest_durchstich import _b             # noqa: E402

BINDUNG = TR.lade_bindung()

# Kz, Person-A-Feld, A-Betrag(Cent), Partner-Feld, Partner-Betrag(Cent) -- Werte aus der bereits
# live gemessenen Messung (/tmp/messe_kostet_ausgang2_geld.py), unterscheidbar je Seite und
# deutlich unter dem § 10 Abs. 4-Hoechstbetrag (2.800 EUR ohne Zuschussanspruch; hier je Kategorie
# hoechstens 980 EUR).
_KATEGORIEN = (
    ("E2001403", "vorsorge_arbeitslosenversicherung", 50000,
     "vorsorge_arbeitslosenversicherung_partner", 48000),
    ("E2001503", "vorsorge_erwerbsunfaehigkeit", 30000,
     "vorsorge_erwerbsunfaehigkeit_partner", 32000),
    ("E2001803", "vorsorge_unfall_haftpflicht", 20000,
     "vorsorge_unfall_haftpflicht_partner", 21000),
    ("E2001903", "vorsorge_rv_alt_mit_ueberschuss", 40000,
     "vorsorge_rv_alt_mit_ueberschuss_partner", 41000),
    ("E2002003", "vorsorge_rv_alt_ohne_ueberschuss", 30000,
     "vorsorge_rv_alt_ohne_ueberschuss_partner", 29000),
)

# BEWUSST kein geliehener "gesamt"-Kegel (weder abgeschrieben noch aus api_constants.SCHEIBEN
# gezogen): deklariere()/erzeuge_xml() sind reines Feld-Routing, keine Ring-Berechnung -- die
# Kegel-Vollstaendigkeitspruefung sitzt ausschliesslich in API._feste_zahl/_an_gesamt_sperrgrund
# (bescheid_deklaration.py), die dieser Test nie aufruft. Geprueft (2026-08-30,
# /tmp/probe_minimal_alle_kategorien.py): alle 5 Kategorien liefern mit NUR "veranlagung"+den
# beiden Zielfeldern dieselben Ergebnisse wie mit dem vollen Kegel. Ein zusaetzliches
# Pflichtfeld in SCHEIBEN["gesamt"]["kegel"] (wie das aktuelle, ungecommittete
# "kein_p23_verkauf") kann diesen Test also nie betreffen, weil er den Kegel gar nicht liest.
_BASIS_KEGEL = (("veranlagung", "zusammen"),)


def _kz_text(root, kz):
    return [el.text for el in root.iter() if el.tag.rsplit("}", 1)[-1] == kz]


def _xml_fuer(fall_id, paare):
    """Baut einen Fall ueber den echten Pfad (materialisiere -> deklariere -> erzeuge_xml) und
    liefert den geparsten Baum -- misst am ECHTEN XML, nicht am flachen deklaration-Dict."""
    store = ST.leerer_store(2025, fall_id=fall_id)
    for feld, wert in _BASIS_KEGEL + tuple(paare):
        _b(store, feld, wert)
    felder, sid = ST.materialisiere(store)
    ergebnis = est_mapping.deklariere(felder, BINDUNG, snapshot_id=sid)
    xml_text = EX.erzeuge_xml(ergebnis, vz=2025, hersteller_id="TESTHID-NICHT-ECHT",
                               abgabefaehig=False)
    return ET.fromstring(xml_text)


@pytest.mark.parametrize("kz, feld_a, cent_a, feld_b, cent_b", _KATEGORIEN)
def test_person_a_allein_traegt_ihren_betrag(kz, feld_a, cent_a, feld_b, cent_b):
    """Nur Person A hat Beitraege in der Kategorie -> das Sum-Kz traegt A's Betrag. Heute
    korrekt -- ein Fix fuer den xfail unten darf diesen Normalfall nicht sperren."""
    root = _xml_fuer(f"ab_lp_nur_a_{kz}", ((feld_a, cent_a),))
    assert _kz_text(root, kz) == [str(cent_a // 100)]


@pytest.mark.xfail(strict=True, reason="A_B_LP/.../Sum ist laut XSD (E10-2025.xsd, "
                    "Weit_Sons_VorAW, kein Person-Diskriminator im Teilbaum) die GEMEINSAME "
                    "Summe beider Ehegatten -- est_mapping.py:654-655 ueberschreibt statt zu "
                    "addieren, wenn beide Seiten einen Betrag in derselben Kategorie haben.")
@pytest.mark.parametrize("kz, feld_a, cent_a, feld_b, cent_b", _KATEGORIEN)
def test_person_a_und_partner_werden_addiert(kz, feld_a, cent_a, feld_b, cent_b):
    """Beide Ehegatten haben Beitraege in DERSELBEN Kategorie, mit unterscheidbaren Betraegen
    (sonst saehe A's Alleinwert im XML wie eine funktionierende Summe aus). Erwartung: das
    Sum-Kz traegt A + Partner. Gemessen (2026-08-30): es traegt nur A."""
    root = _xml_fuer(f"ab_lp_a_und_partner_{kz}", ((feld_a, cent_a), (feld_b, cent_b)))
    assert _kz_text(root, kz) == [str((cent_a + cent_b) // 100)]
