"""Nagelt eine Asymmetrie fest: `/deklaration` fragt `_an_gesamt_sperrgrund` nie, `/ergebnis`
und `/einreichen` fragen ihn (api.py Zeile 570 bzw. 699). Derselbe Fall (reiner Person-A-Kegel,
kein Partner -- kein Partnerloch), dieselbe kapital_semantik_offen-Kollision (Aggregat
kap_kapitalertraege UND Topf kap_gewinn_aktien gleichzeitig bestaetigt):

- GET /ergebnis   -> gesperrt (grund=kapital_semantik_offen, zahl_cent=None)      [GRUENE KONTROLLE]
- POST /einreichen -> gesperrt (409, grund=kapital_semantik_offen)                [GRUENE KONTROLLE]
- GET /deklaration -> HTTP 200, vollstaendig=True, eingaben_konsistent=True,
  UND zeigt Aggregat (E1900701) und Topf (E1900901) gleichzeitig                  [XFAIL, STRICT]

Die beiden gruenen Kontrollen sind Pflicht, nicht Dekoration: ohne sie waere ein "Deklaration
liefert 200" auch mit einem harmlosen Fall zu bekommen, der den Waechter gar nicht auslöst.
Erst wenn ergebnis UND einreichen auf DEMSELBEN Fall wirklich sperren, beweist der dritte
Befund die Luecke, nicht einen falschen Testfall.

Reichweite -- wörtlich aus der Recherche uebernommen, nicht aus der Serverseite geschlossen:

`produkt/haut/static/app.js` -- der einzige mit diesem Server gepaarte, tatsaechlich
ausgelieferte Client (Grep case-insensitive auf den String "deklaration" in der gesamten
Datei: 0 Treffer) -- ruft `/deklaration` an KEINER Stelle auf. Verdrahtet sind dort nur
`zeigeErgebnis()` -> GET /fall/{id}/ergebnis (normaler Anzeige-Pfad) und `einreichenPruefen()`
-> POST /fall/{id}/einreichen, gebunden an den Klick-Handler des Absende-Buttons
("einreichen-btn"). Eine repo-weite Suche nach dem LITERALEN Pfad "/deklaration" (nicht dem
Wort "deklaration", das u.a. in "deklariere"/"eingaben_konsistent"-Nachbarschaft zu breit
matcht) trifft genau sechs Dateien: vier Testdateien, `produkt/haut/server.py` (die
Routenregistrierung selbst) und `produkt/haut/KONZEPT.md` (die urspruengliche
Konzept-Skizze, dort Zeile 45 als "ELSTER-Deklarationsvorschau (Store->E-Nr),
lossy-transparent" dokumentiert) -- keine einzige weitere Client- oder Export-Datei
(auch `pipeline/ui/static/index.html`, eine zweite, GEPRUEFTE UND ausgeschlossene
UI-Oberflaeche im selben Repo, enthaelt weder den String "deklaration" noch "/fall/").
Der Endpunkt ist nach diesem Befund AKTUELL NICHT nutzersichtbar -- ein latenter Defekt,
kein akuter Nutzerpfad ueber die ausgelieferte Oberflaeche.

Der Eingabezustand, den er falsch behandelt, ist dagegen sehr wohl ueber den normalen
Frage-Fluss herstellbar, nicht nur ueber direkte API-Manipulation: `kap_kapitalertraege`
(E1900701, Aggregat) und `kap_gewinn_aktien` (E1900901, Aktien-Topf) tragen in
`produkt/bindung/bindung_kap_vv_familie.yaml` KEINE gegenseitige feld_bedingung -- beide
sind unbedingt und unabhaengig voneinander askable (die Person-B-Spiegelfelder sind beide
nur an dieselbe Bedingung `kein_kap_partner==false` gekoppelt, nicht aneinander). Ein
Nutzer kann also im ganz normalen Interview beide Fragen wörtlich beantworten und damit
genau den Widerspruch herstellen, den `/ergebnis` und `/einreichen` zu Recht sperren. Sollte
`/deklaration` je an eine Oberflaeche verdrahtet werden -- die Konzept-Skizze benennt genau
diese Absicht ("so sieht deine Erklaerung aus") --, wuerde der Defekt beim ersten echten
Nutzer sichtbar, der beide KAP-Fragen wahrheitsgemaess beantwortet.

Was dieser Test NICHT behauptet: dass ein Nutzer diesen Widerspruch heute in der Oberflaeche
je zu sehen bekommt (er bekommt ihn nicht, s.o.); dass dies ein Partner-spezifisches Loch ist
(reiner Person-A-Fall hier, bewusst ohne veranlagung=zusammen); dass der Fix in `deklaration()`
liegen muss statt in einer gemeinsamen Vorpruef-Funktion -- das ist eine Entscheidung ueber
den Instructor/Julius, keine, die dieser Test trifft oder vorwegnimmt. Nicht repariert.
"""
from __future__ import annotations

import os
import sys

import pytest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from test_paket_b_e2e_http import base, _req, _gesamt_kegel, _gesamt_anlegen  # noqa: F401,E402


FALL = "deklaration_waechter_gepinnt"


def _fall_mit_kapital_semantik_offen(base_url):
    """Reiner Person-A-Kegel: Aggregat UND Aktien-Topf gleichzeitig bestaetigt, kein Partner."""
    _gesamt_anlegen(base_url, FALL, _gesamt_kegel(
        0, kein_vuv=True, kein_kap=False,
        kap_ertraege=500000, kap_gewinn_aktien=300000))


def test_ergebnis_sperrt_bei_kapital_semantik_offen(base):
    """GRUENE KONTROLLE 1: /ergebnis erkennt den Widerspruch und liefert keine Zahl."""
    _fall_mit_kapital_semantik_offen(base)
    st, erg = _req(base, "GET", f"/fall/{FALL}/ergebnis")
    assert st == 200
    assert erg["grund"] == "kapital_semantik_offen", f"Ergebnis hat NICHT gesperrt: {erg}"
    assert erg["zahl_cent"] is None


def test_einreichen_sperrt_bei_kapital_semantik_offen(base):
    """GRUENE KONTROLLE 2: /einreichen erkennt denselben Widerspruch, 409, VOR EM.deklariere."""
    _fall_mit_kapital_semantik_offen(base)
    st, res = _req(base, "POST", f"/fall/{FALL}/einreichen", {}, erwarte=409)
    assert res["grund"] == "kapital_semantik_offen", f"Einreichen hat NICHT gesperrt: {res}"
    assert res["eingereicht"] is False


@pytest.mark.xfail(
    strict=True,
    reason="BUG (nicht Testfehler): api.py deklaration() ruft _an_gesamt_sperrgrund nie auf "
           "(anders als _ergebnis_roh Zeile 570 und einreichen Zeile 699) -- deklariert einen "
           "Widerspruch als vollstaendig statt ihn zu sperren. Faellt dieser xfail um, weil die "
           "Asymmetrie behoben wurde: Marker entfernen, Kontrollen oben bleiben.",
)
def test_deklaration_erkennt_widerspruch_NICHT_bug_gepinnt(base):
    """Erwartung, die HEUTE nicht zutrifft: Deklaration darf nicht gleichzeitig vollstaendig UND
    Aggregat+Topf gleichzeitig melden, wenn ergebnis/einreichen denselben Fall sperren."""
    _fall_mit_kapital_semantik_offen(base)
    st, dek = _req(base, "GET", f"/fall/{FALL}/deklaration")
    assert st == 200  # heute IMMER 200, der Waechter wird nie gefragt
    aggregat = dek["deklaration"].get("E1900701")
    topf = dek["deklaration"].get("E1900901")
    assert not (dek.get("vollstaendig") and aggregat and topf), (
        f"Deklaration zeigt Aggregat({aggregat}) UND Topf({topf}) gleichzeitig bei "
        f"vollstaendig={dek.get('vollstaendig')}, eingaben_konsistent={dek.get('eingaben_konsistent')} "
        f"-- derselbe Fall, den ergebnis/einreichen zu Recht sperren."
    )
