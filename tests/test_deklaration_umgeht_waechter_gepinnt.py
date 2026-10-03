"""Haelt fest, dass `/deklaration` den Sperrgrund wie `/ergebnis` und `/einreichen` fragt
(`_an_gesamt_sperrgrund`, Julius 2026-10-03, decisions/deklaration-darf-verweigern.md). Bis dahin
fragte `deklaration()` ihn nie, und dieser Test war ein `xfail(strict)` auf die Asymmetrie. Derselbe
Fall (reiner Person-A-Kegel, kein Partner -- kein Partnerloch), dieselbe kapital_semantik_offen-
Kollision (Aggregat kap_kapitalertraege UND Topf kap_gewinn_aktien gleichzeitig bestaetigt):

- GET /ergebnis   -> gesperrt (grund=kapital_semantik_offen, zahl_cent=None)      [GRUENE KONTROLLE]
- POST /einreichen -> gesperrt (409, grund=kapital_semantik_offen)                [GRUENE KONTROLLE]
- GET /deklaration -> gesperrt (409, grund=kapital_semantik_offen, klartext wie /ergebnis,
  keine Kz im Koerper)                                                             [DIE SPERRE]

Die beiden Kontrollen sind Pflicht, nicht Dekoration: ohne sie waere ein "Deklaration sperrt"
auch mit einem Fall zu bekommen, der aus einem anderen Grund sperrt. Erst wenn ergebnis UND
einreichen auf DEMSELBEN Fall mit DEMSELBEN Grund sperren, beweist der dritte Test die Sperre.
Faellt der Aufruf in `deklaration()` weg, wird dieser dritte Test rot (409 erwartet, 200 geliefert);
ein `xfail` ohne `raises=` haette das nicht gemeldet (`_req` wirft bei 4xx, der Marker nimmt jeden
Fehler als erwartet).

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
Der Endpunkt ist nach diesem Befund AKTUELL NICHT nutzersichtbar -- die Sperre aendert fuer
niemanden etwas Sichtbares, bis jemand ihn an eine Oberflaeche anschliesst.

Der Eingabezustand, den er falsch behandelt, ist dagegen sehr wohl ueber den normalen
Frage-Fluss herstellbar, nicht nur ueber direkte API-Manipulation: `kap_kapitalertraege`
(E1900701, Aggregat) und `kap_gewinn_aktien` (E1900901, Aktien-Topf) tragen in
`produkt/bindung/bindung_kap_vv_familie.yaml` KEINE gegenseitige feld_bedingung -- beide
sind unbedingt und unabhaengig voneinander askable (die Person-B-Spiegelfelder sind beide
nur an dieselbe Bedingung `kein_kap_partner==false` gekoppelt, nicht aneinander). Ein
Nutzer kann also im ganz normalen Interview beide Fragen wörtlich beantworten und damit
genau den Widerspruch herstellen, den `/ergebnis` und `/einreichen` zu Recht sperren. Wird
`/deklaration` je an eine Oberflaeche verdrahtet -- die Konzept-Skizze benennt genau diese
Absicht ("so sieht deine Erklaerung aus") --, bekommt ein Nutzer, der beide KAP-Fragen
wahrheitsgemaess beantwortet, die Sperre mit Klartext statt zweier widerspruechlicher Betraege.

Was dieser Test NICHT behauptet: dass ein Nutzer diesen Widerspruch heute in der Oberflaeche
je zu sehen bekommt (er bekommt ihn nicht, s.o.); dass dies ein Partner-spezifisches Loch ist
(reiner Person-A-Fall hier, bewusst ohne veranlagung=zusammen); dass die Sperre jeden Fall
erfasst, in dem `/deklaration` Werte aus unbestaetigten Feldern zeigt -- nur Faelle, in denen
`_an_gesamt_sperrgrund` einen Grund meldet (s. test_kap_deklaration_vorlaeufig_leck_ohne_
bestaetigung.py: dort meldet er keinen).
"""
from __future__ import annotations

import os
import sys

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


def test_deklaration_sperrt_bei_kapital_semantik_offen(base):
    """DIE SPERRE: derselbe Fall, 409 mit demselben Grund, dem Satz von /ergebnis und ohne Kz."""
    _fall_mit_kapital_semantik_offen(base)
    _, erg = _req(base, "GET", f"/fall/{FALL}/ergebnis")
    st, dek = _req(base, "GET", f"/fall/{FALL}/deklaration", erwarte=409)
    assert dek["grund"] == "kapital_semantik_offen", f"Deklaration hat NICHT gesperrt: {dek}"
    assert dek["klartext"] and dek["klartext"] == erg["klartext"], (dek, erg.get("klartext"))
    # Der Koerper ist genau `fall_id`, `grund`, `klartext` -- kein Aggregat, kein Topf, kein Kz.
    assert set(dek) == {"fall_id", "grund", "klartext"}, dek
    assert dek["fall_id"] == FALL
