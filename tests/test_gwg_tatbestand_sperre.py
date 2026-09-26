"""§ 6 Abs. 2 EStG: die drei GWG-Anspruchsvoraussetzungen sperren die Zahl, wenn sie offen sind.

BEFUND 2026-09-08 (Mutationsprobe, deshalb existiert diese Datei): der Sperrgrund
`gwg_tatbestand_offen` (bescheid_deklaration.py::_an_gesamt_sperrgrund) war GAR NICHT bewacht.
Der ganze Block liess sich stilllegen (`if False and store is not None ...`) und die volle Suite
blieb Zeichen fuer Zeichen gleich gruen -- 3073 gruen, dieselbe eine vorbestehende Rote. Der Name
kam in Tests nur in KOMMENTAREN vor, in keinem einzigen Assert; die vier Tests, die ihn erwaehnen,
beantworten die drei Fragen gerade so, dass die Sperre NICHT feuert. Damit war die zweite Haelfte
des GWG-Umbaus ungeprueft: die Nullung bei verneintem Tatbestand (bescheid_einkuenfte.py::_abzug)
ist bewacht -- stillgelegt wird test_gesamt_gwg_ohne_tatbestand_darf_keinen_sofortabzug_geben rot --,
die Sperre bei UNBEANTWORTETEM Tatbestand war es nicht.

Was hier festgehalten wird, sind vier Zusagen, die bisher nur als Prosa im Code standen:

  1. offen sperrt          -- ein bestaetigter Betrag ohne Antwort auf die Voraussetzungen macht
                              die Zahl unhaltbar. Ohne diese Sperre flosse der Sofortabzug, ohne
                              dass je jemand bestaetigt hat, dass er zusteht (Under-tax).
  2. "nein" sperrt NICHT   -- eine verneinte Voraussetzung ist eine ANTWORT. Der Ring ist dann
                              rechenbar, das Wirtschaftsgut gehoert in die AfA (_abzug nullt es).
                              Wuerde auch "nein" sperren, kaeme der ehrliche Nutzer nie durch.
  3. ueber 800 EUR         -- S. 1 schliesst den Sofortabzug am BETRAG aus. Die drei Fragen sind
     sperrt NICHT             fuer dieses Wirtschaftsgut gegenstandslos; nach Voraussetzungen zu
                              fragen, die am Betrag laengst gescheitert sind, sperrt die Abgabe
                              grundlos (im Code als Messung vom 2026-09-07 vermerkt, ungepinnt).
  4. unter 250 EUR         -- die Verzeichnispflicht aus S. 4 gilt erst darueber. Darunter darf
     verlangt kein            ihre fehlende Antwort nicht sperren.
     Verzeichnis

Zu 3. und 4.: das sind die beiden Ausnahmen, bei denen ein zu breiter Guard NICHT falsch rechnet,
sondern den Nutzer aussperrt -- der teurere Fehler, weil er wie Vorsicht aussieht.

Gemessen wird ueber API.ergebnis() statt ueber HTTP: der Sperrgrund entsteht in
bescheid_deklaration.py und kommt unveraendert als `grund` heraus, ein Socket dazwischen wuerde
nichts zusaetzlich belegen. Kegel und Helfer sind bewusst eigene (Vorbild
tests/test_instanz_luecke.py) statt Import aus tests/test_stille_null_offen.py.

Die Kontrollfaelle pruefen `grund == "bestaetigt"`, nicht `grund != "gwg_tatbestand_offen"`:
sonst geht ein Test gruen durch, weil ein GANZ ANDERER Sperrgrund gefeuert hat, und der
Kontrastfall traegt nichts.

NULL LLM."""
from __future__ import annotations

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "golden"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api as API        # noqa: E402
import audit             # noqa: E402

from _kegel import kegel_fuer  # noqa: E402

# Betraege in CENT. 50000 = 500 EUR liegt zwischen beiden Schwellen (> 250, <= 800): dort gelten
# ALLE DREI Fragen. 100000 = 1000 EUR ist ueber der 800-EUR-Grenze aus S. 1, 20000 = 200 EUR unter
# der 250-EUR-Verzeichnisgrenze aus S. 4.
MITTLERES_GWG = 50000
UEBER_800 = 100000
UNTER_250 = 20000

IMMER_NOETIG = ("gwg_bewegliches_selbstaendig_nutzbar", "gwg_netto_ohne_vorsteuer")
AB_250 = "gwg_verzeichnis_ab_250"

# GEBAUT, nicht kopiert (tests/_kegel.py), seit 2026-09-26. Von Hand standen hier 34 Felder;
# 30 trugen genau den Abwesenheitswert. Nur die vier unten sind echte Werte.
#
# Die beiden agb-Felder bleiben AUSDRUECKLICH auf True -- das ist kein Abwesenheitswert, sondern
# eine Entscheidung: sie halten die Regel `p33_1_2_agb_abzug` im Kegel offen, ohne die
# gwg-Messung zu beruehren. Der Bauer wuerde dort False setzen (Name ohne "kein_"), die Regel
# waere ausgeschlossen, und der Test mae sse etwas anderes. Vorher standen sie als handkopierte
# Zeilen mit genau dieser Begruendung hier -- jetzt als Override, mit derselben Begruendung.
_KEGEL = kegel_fuer("gesamt", {
    "bruttoarbeitslohn": 6000000,
    "vv_entgelt_quote_prozent": 100,
    "kein_gewinn": False,
    "agb_zwangslaeufig": True,
    "agb_notwendig_angemessen": True,
})


def _laie(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie",
            "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


def _fall_mit_gwg(tmp_path, monkeypatch, fid, netto_cent, antworten=()):
    """Legt einen 'gesamt'-Fall mit genau EINER gwg-Instanz an und beantwortet `antworten`
    (Paare feld_id/wert). Alles andere bleibt unbeantwortet -- das ist der Messgegenstand."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    st, r = API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fid})
    assert st == 201, r
    for feld, wert in _KEGEL:
        st, r = API.event(fid, _laie(feld, wert))
        assert st == 201, f"{feld}={wert}: {st} {r}"
    st, r = API.event(fid, _laie("gwg_anschaffungskosten_netto", netto_cent))
    assert st == 201, r
    for feld, wert in antworten:
        st, r = API.event(fid, _laie(feld, wert))
        assert st == 201, f"{feld}={wert}: {st} {r}"
    st, erg = API.ergebnis(fid)
    assert st == 200, erg
    return erg


def test_unbeantworteter_tatbestand_sperrt(tmp_path, monkeypatch):
    """500 EUR bestaetigt, keine der drei Voraussetzungen beantwortet -> die Zahl ist unhaltbar.
    Ohne diese Sperre flosse der Sofortabzug, ohne dass der Anspruch je bestaetigt wurde."""
    erg = _fall_mit_gwg(tmp_path, monkeypatch, "gwg-sperre-offen", MITTLERES_GWG)
    assert erg["grund"] == "gwg_tatbestand_offen", (
        f"unbeantwortete § 6 Abs. 2-Voraussetzungen muessen sperren, grund={erg['grund']!r}")
    assert erg["zahl_cent"] is None, (
        f"gesperrt heisst keine Zahl, sonst ist die Sperre nur Dekoration: {erg['zahl_cent']}")


def test_eine_offene_von_dreien_reicht(tmp_path, monkeypatch):
    """Zwei von drei beantwortet reicht NICHT -- sonst haengt die Sperre an der ersten Frage und
    die anderen beiden waeren wirkungslos (der Fehler, der bei einer &&-Kette leicht passiert)."""
    erg = _fall_mit_gwg(tmp_path, monkeypatch, "gwg-sperre-teil", MITTLERES_GWG,
                        antworten=[(f, True) for f in IMMER_NOETIG])
    assert erg["grund"] == "gwg_tatbestand_offen", (
        f"das offene Verzeichnis-Feld allein muss sperren, grund={erg['grund']!r}")


def test_verneinter_tatbestand_sperrt_nicht(tmp_path, monkeypatch):
    """"Nein" ist eine ANTWORT, keine Luecke: der Ring ist rechenbar, das Geraet gehoert in die
    AfA (_gwg_sofortabzug_summe nullt genau diese Instanz). Wuerde auch "nein" sperren, kaeme der
    ehrliche Nutzer nie durch -- fail-closed am falschen Ort."""
    erg = _fall_mit_gwg(tmp_path, monkeypatch, "gwg-sperre-nein", MITTLERES_GWG,
                        antworten=[("gwg_bewegliches_selbstaendig_nutzbar", False),
                                   ("gwg_netto_ohne_vorsteuer", True),
                                   (AB_250, True)])
    assert erg["grund"] == "bestaetigt", (
        f"eine verneinte Voraussetzung ist beantwortet und darf nicht sperren, grund={erg['grund']!r}")


def test_ueber_800_euro_fragt_nicht_nach_dem_tatbestand(tmp_path, monkeypatch):
    """§ 6 Abs. 2 S. 1: ueber 800 EUR netto ist der Sofortabzug am BETRAG ausgeschlossen (zwingend
    AfA). Die drei Fragen sind fuer dieses Wirtschaftsgut gegenstandslos -- unbeantwortet duerfen
    sie die Abgabe nicht sperren, sonst verlangt das Programm Angaben zu einem Anspruch, den es
    selbst schon abgelehnt hat."""
    erg = _fall_mit_gwg(tmp_path, monkeypatch, "gwg-sperre-ueber800", UEBER_800)
    assert erg["grund"] == "bestaetigt", (
        f"1000-EUR-Geraet: kein Sofortabzug moeglich, also nichts zu fragen, grund={erg['grund']!r}")


def test_unter_250_euro_verlangt_kein_verzeichnis(tmp_path, monkeypatch):
    """§ 6 Abs. 2 S. 4: die Aufzeichnungspflicht gilt erst ueber 250 EUR netto. Darunter darf die
    fehlende Antwort auf die Verzeichnisfrage nicht sperren -- die beiden anderen Voraussetzungen
    gelten weiterhin und sind hier beantwortet."""
    erg = _fall_mit_gwg(tmp_path, monkeypatch, "gwg-sperre-unter250", UNTER_250,
                        antworten=[(f, True) for f in IMMER_NOETIG])
    assert erg["grund"] == "bestaetigt", (
        f"200-EUR-Geraet braucht kein Verzeichnis, grund={erg['grund']!r}")
