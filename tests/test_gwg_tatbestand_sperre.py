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
  2. "nein" ist eine ANTWORT -- (bis 2026-10-03: "sperrt NICHT", der Ring nullte das Geraet still;
                              ersetzt, siehe NACHTRAG unten.)
  3. ueber 800 EUR         -- S. 1 schliesst den Sofortabzug am BETRAG aus. Die drei Fragen sind
     fragt nicht nach         fuer dieses Wirtschaftsgut gegenstandslos; nach Voraussetzungen zu
     dem Tatbestand           fragen, die am Betrag laengst gescheitert sind, sperrt die Abgabe
                              grundlos (Messung vom 2026-09-07). Das Geraet selbst verschwindet
                              seit 2026-10-03 nicht mehr still, siehe NACHTRAG unten.
  4. unter 250 EUR         -- die Verzeichnispflicht aus S. 4 gilt erst darueber. Darunter darf
     verlangt kein            ihre fehlende Antwort nicht sperren.
     Verzeichnis

Zu 3. und 4.: das sind die beiden Ausnahmen, bei denen ein zu breiter Guard NICHT falsch rechnet,
sondern den Nutzer aussperrt -- der teurere Fehler, weil er wie Vorsicht aussieht.

NACHTRAG 2026-10-03 (main-Auftrag h8-gwg, Eintrag gwg-sofortabzug-entfaellt-ohne-nettobetrag,
AK2/AK3, erweitert um gwg-ohne-verzeichnis-verschwindet-statt-abgeschrieben): jede Konstellation, in
der ein Geraet mit Betrag > 0 KEINEN Sofortabzug bekommt, war bis dahin `grund=bestaetigt` mit einem
stillen Abzug von 0 (gemessen auf 9ad51c19: 790 EUR + "netto: nein" -> 1392400 Cent, Fall ohne GWG;
Referenz 790 EUR als sonstige Betriebsausgabe 1362000, also 304,00 EUR zu viel Steuer). Seitdem ist sie
sichtbar OFFEN, mit zwei neuen Gruenden:
  - `gwg_mehrwertsteuer_offen`  -- "netto: nein" (bestaetigt): ob und wie viel abzugsfaehig ist, haengt
                                   an der Vorsteuer (Kleinunternehmer: Bruttobetrag). Die Folgefrage, die
                                   das entscheidet, gibt es noch nicht (Entscheidung
                                   gwg-ohne-vorsteuerabzug-zieht-den-bruttobetrag-ab, AK1/AK4).
  - `gwg_abschreibung_offen`   -- das Geraet ist kein GWG (nicht selbstaendig nutzbar, ueber 800 EUR,
                                   ueber 250 EUR ohne Verzeichnis): es gehoert in die AfA, die hier nicht
                                   gerechnet wird.
"nein" ist damit weiter eine Antwort, aber keine, mit der eine Zahl entsteht. Der Ausweg des Nutzers:
Betrag auf 0 setzen und das Geraet bei der Abschreibung eintragen (test_betrag_null_...).
Rechnung (bescheid_einkuenfte._gwg_sofortabzug_summe) und Sperre (_an_gesamt_sperrgrund) bleiben je ein
Stueck Code; dass sie dasselbe Urteil liefern, prueft test_offen_an_allen_drei_stellen_gleich.

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

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "golden"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api as API        # noqa: E402
import audit             # noqa: E402
import bescheid_deklaration as BD  # noqa: E402

from _kegel import kegel_fuer  # noqa: E402

# Betraege in CENT. 50000 = 500 EUR liegt zwischen beiden Schwellen (> 250, <= 800): dort gelten
# ALLE DREI Fragen. 100000 = 1000 EUR ist ueber der 800-EUR-Grenze aus S. 1, 20000 = 200 EUR unter
# der 250-EUR-Verzeichnisgrenze aus S. 4.
MITTLERES_GWG = 50000
UEBER_800 = 100000
UNTER_250 = 20000

IMMER_NOETIG = ("gwg_bewegliches_selbstaendig_nutzbar", "gwg_netto_ohne_vorsteuer")
AB_250 = "gwg_verzeichnis_ab_250"
GRUND_MWST = "gwg_mehrwertsteuer_offen"
GRUND_ABSCHREIBUNG = "gwg_abschreibung_offen"

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
    # GWG-Zeile eröffnet den EÜR-Weg; ohne bestätigte EÜR-Angaben sperrt gewinn_angaben_offen (2026-09-28).
    "betriebseinnahmen": 0, "sonstige_betriebsausgaben": 0, "afa_jahresbetrag": 0,
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


def test_verneinter_tatbestand_ist_offen_mit_grund_nicht_stille_null(tmp_path, monkeypatch):
    """"Nein" bei "allein benutzbar" ist eine ANTWORT, und sie macht das Geraet zu einem Fall fuer die
    AfA. Bis 2026-10-03 nullte _gwg_sofortabzug_summe die Instanz still und das Ergebnis blieb
    `bestaetigt` (gemessen 500 EUR: 1392400 Cent = Fall ganz ohne GWG, gegen 1388500 mit dem
    Fuenftel als Betriebsausgabe). Heute steht der Fall sichtbar offen: gleiche Antwort, aber keine
    Zahl, die so tut, als sei der Abzug vollstaendig."""
    erg = _fall_mit_gwg(tmp_path, monkeypatch, "gwg-sperre-nein", MITTLERES_GWG,
                        antworten=[("gwg_bewegliches_selbstaendig_nutzbar", False),
                                   ("gwg_netto_ohne_vorsteuer", True),
                                   (AB_250, True)])
    assert erg["grund"] == GRUND_ABSCHREIBUNG, (
        f"ein verneinter Tatbestand darf nicht still 0 abziehen, grund={erg['grund']!r}")
    assert erg["zahl_cent"] is None, erg


def test_ueber_800_euro_fragt_nicht_nach_dem_tatbestand_und_ist_offen(tmp_path, monkeypatch):
    """§ 6 Abs. 2 S. 1: ueber 800 EUR netto ist der Sofortabzug am BETRAG ausgeschlossen (zwingend
    AfA). Die drei Fragen sind fuer dieses Wirtschaftsgut gegenstandslos -- unbeantwortet duerfen
    sie die Abgabe nicht sperren, sonst verlangt das Programm Angaben zu einem Anspruch, den es
    selbst schon abgelehnt hat."""
    erg = _fall_mit_gwg(tmp_path, monkeypatch, "gwg-sperre-ueber800", UEBER_800)
    # Seit 2026-10-03 nicht mehr `bestaetigt`: das Geraet fragt weiter nichts (kein gwg_tatbestand_offen,
    # die Fragen waeren gegenstandslos), verschwindet aber auch nicht mehr still -- es gehoert in die AfA.
    assert erg["grund"] == GRUND_ABSCHREIBUNG, (
        f"1000-EUR-Geraet: kein Sofortabzug, also nichts zu fragen UND nicht still weglassen, "
        f"grund={erg['grund']!r}")
    assert erg["grund"] != "gwg_tatbestand_offen"


def test_unter_250_euro_verlangt_kein_verzeichnis(tmp_path, monkeypatch):
    """§ 6 Abs. 2 S. 4: die Aufzeichnungspflicht gilt erst ueber 250 EUR netto. Darunter darf die
    fehlende Antwort auf die Verzeichnisfrage nicht sperren -- die beiden anderen Voraussetzungen
    gelten weiterhin und sind hier beantwortet."""
    erg = _fall_mit_gwg(tmp_path, monkeypatch, "gwg-sperre-unter250", UNTER_250,
                        antworten=[(f, True) for f in IMMER_NOETIG])
    assert erg["grund"] == "bestaetigt", (
        f"200-EUR-Geraet braucht kein Verzeichnis, grund={erg['grund']!r}")


# ------------------------------------------------------------------ kein stiller Abzug von 0
#
# main-Auftrag h8-gwg, 2026-10-03. Jede Zeile: (Name, Betrag in Cent, nutzbar, netto, verzeichnis, Grund).
# None = nicht beantwortet. Der Grund ist der, den ALLE drei Stellen (ergebnis, deklaration, einreichen)
# liefern muessen.
OFFEN_FAELLE = [
    # AK2: "netto: nein" -- der Kleinunternehmer mit Bruttobetrag, die Folgefrage gibt es noch nicht.
    ("790 brutto, netto=nein", 79000, True, False, True, GRUND_MWST),
    ("500 brutto, netto=nein", MITTLERES_GWG, True, False, True, GRUND_MWST),
    # AK3: Betrag ueber 800 EUR mit "netto: nein" (Band 800,01-952 EUR bei 19 %, nicht rechenbar).
    ("850 brutto, netto=nein", 85000, True, False, True, GRUND_MWST),
    ("1000 brutto, netto=nein", UEBER_800, True, False, True, GRUND_MWST),
    # erweitert um gwg-ohne-verzeichnis-verschwindet-statt-abgeschrieben: kein GWG -> AfA, nicht gerechnet.
    ("500, Verzeichnis=nein", MITTLERES_GWG, True, True, False, GRUND_ABSCHREIBUNG),
    ("500, nicht selbstaendig nutzbar", MITTLERES_GWG, False, True, True, GRUND_ABSCHREIBUNG),
    ("500, nicht nutzbar UND netto=nein (MwSt ist egal, kein GWG)", MITTLERES_GWG, False, False, True,
     GRUND_ABSCHREIBUNG),
    ("800,01 netto", 80001, True, True, True, GRUND_ABSCHREIBUNG),
    ("1000 netto, alles ja", UEBER_800, True, True, True, GRUND_ABSCHREIBUNG),
    ("1000, Fragen unbeantwortet", UEBER_800, None, None, None, GRUND_ABSCHREIBUNG),
    ("250,01 ohne Verzeichnis", 25001, True, True, False, GRUND_ABSCHREIBUNG),
]


def _antworten(nutzbar, netto, verzeichnis):
    paare = (("gwg_bewegliches_selbstaendig_nutzbar", nutzbar), ("gwg_netto_ohne_vorsteuer", netto),
             (AB_250, verzeichnis))
    return [(f, w) for f, w in paare if w is not None]


def _fall_ohne_gwg(tmp_path, monkeypatch, fid, sonstige_cent=0):
    """Referenzfall: derselbe Kegel ohne GWG-Zeile, der Betrag steht als sonstige Betriebsausgabe."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    st, r = API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fid})
    assert st == 201, r
    for feld, wert in _KEGEL:
        if feld == "sonstige_betriebsausgaben":
            wert = sonstige_cent
        st, r = API.event(fid, _laie(feld, wert))
        assert st == 201, f"{feld}={wert}: {st} {r}"
    st, erg = API.ergebnis(fid)
    assert st == 200, erg
    return erg


@pytest.mark.parametrize("name,betrag,nutzbar,netto,verzeichnis,grund", OFFEN_FAELLE,
                         ids=[f[0] for f in OFFEN_FAELLE])
def test_offen_an_allen_drei_stellen_gleich(tmp_path, monkeypatch, name, betrag, nutzbar, netto,
                                            verzeichnis, grund):
    """Ein Geraet mit Betrag > 0 ohne Sofortabzug ist sichtbar OFFEN: keine Zahl, der Grund, sein
    Klartext. /ergebnis, /deklaration und /einreichen liefern dasselbe Urteil -- sonst driften Rechnung
    und Abgabesperre auseinander (Entscheidung gwg-ohne-vorsteuerabzug-zieht-den-bruttobetrag-ab, 5.)."""
    fid = "gwg-offen"
    erg = _fall_mit_gwg(tmp_path, monkeypatch, fid, betrag, antworten=_antworten(nutzbar, netto, verzeichnis))
    assert erg["grund"] == grund, f"{name}: grund={erg['grund']!r}, erwartet {grund!r}"
    assert erg["zahl_cent"] is None, f"{name}: gesperrt heisst keine Zahl, sonst ist es ein stiller Abzug: {erg}"
    assert erg["klartext"] == BD.SPERRGRUND_KLARTEXT[grund], erg
    st, dek = API.deklaration(fid)
    assert (st, dek.get("grund")) == (409, grund), f"{name}: /deklaration {st} {dek}"
    st, ein = API.einreichen(fid, {})
    assert (st, ein.get("grund")) == (409, grund), f"{name}: /einreichen {st} {ein}"


def test_790_euro_netto_ja_ist_der_referenzfall(tmp_path, monkeypatch):
    """Kontrolle gegen die Blindheit der Messung: mit "netto: ja" wirkt der Sofortabzug, und die Steuer
    ist gleich der des Falls ohne GWG mit denselben 790 EUR als sonstige Betriebsausgabe. Ohne diese
    Kontrolle waere "offen" im Fall "nein" auch dann gruen, wenn die Sofortabzug-Rechnung gar nichts tut."""
    ohne = _fall_ohne_gwg(tmp_path, monkeypatch, "gwg-ohne", 0)
    ref = _fall_ohne_gwg(tmp_path, monkeypatch, "gwg-ref", 79000)
    ja = _fall_mit_gwg(tmp_path, monkeypatch, "gwg-ja", 79000, antworten=_antworten(True, True, True))
    assert ref["zahl_cent"] != ohne["zahl_cent"], "Messung blind: Referenz == Fall ohne Abzug"
    assert ja["grund"] == "bestaetigt", ja
    assert ja["zahl_cent"] == ref["zahl_cent"], (
        f"790 EUR netto: Sofortabzug muss gleich der Betriebsausgabe sein, {ja['zahl_cent']} != {ref['zahl_cent']}")


@pytest.mark.parametrize("name,betrag,nutzbar,netto,verzeichnis", [
    ("800,00 alles ja", 80000, True, True, True),
    ("250,00 ohne Verzeichnis (die Pflicht beginnt darueber)", 25000, True, True, False),
    ("200 ohne Verzeichnis", UNTER_250, True, True, False),
])
def test_grenzen_bleiben_sofortabzug(tmp_path, monkeypatch, name, betrag, nutzbar, netto, verzeichnis):
    """Gegenprobe zu OFFEN_FAELLE von der anderen Seite: genau an den Grenzen (800,00 / 250,00 EUR)
    bleibt der Sofortabzug, und die Zahl steht. Ein zu breiter Guard sperrt den ehrlichen Nutzer aus."""
    erg = _fall_mit_gwg(tmp_path, monkeypatch, "gwg-grenze", betrag,
                        antworten=_antworten(nutzbar, netto, verzeichnis))
    assert erg["grund"] == "bestaetigt", f"{name}: grund={erg['grund']!r}"
    assert erg["zahl_cent"] is not None


def test_betrag_null_ist_der_ausweg(tmp_path, monkeypatch):
    """Wer das Geraet nicht als Sofortabzug fuehren kann oder will, setzt den Betrag auf 0 und traegt es
    bei der Abschreibung ein: ohne Betrag gibt es nichts wegzulassen, "netto: nein" sperrt dann nicht."""
    erg = _fall_mit_gwg(tmp_path, monkeypatch, "gwg-null", 0,
                        antworten=_antworten(True, False, True))
    assert erg["grund"] == "bestaetigt", f"Betrag 0 darf nicht sperren, grund={erg['grund']!r}"
    assert erg["zahl_cent"] is not None


def test_vorlaeufiges_nein_ist_keine_antwort(tmp_path, monkeypatch):
    """Ein Vorjahres-Vorschlag "netto: nein" (zustand vorlaeufig) ist noch keine Antwort des Nutzers: der
    Fall sperrt wie ein unbeantworteter (`gwg_tatbestand_offen`), nicht mit `gwg_mehrwertsteuer_offen`.
    Haelt die Zeile fest, dass nur ein BESTAETIGTES "nein" ein Urteil ist (Zwei-Signal-Regel)."""
    fid = "gwg-vorlaeufig-nein"
    erg = _fall_mit_gwg(tmp_path, monkeypatch, fid, MITTLERES_GWG,
                        antworten=[("gwg_bewegliches_selbstaendig_nutzbar", True), (AB_250, True)])
    assert erg["grund"] == "gwg_tatbestand_offen", erg      # Ausgangslage: eine Frage offen
    st, r = API.event(fid, {"feld_id": "gwg_netto_ohne_vorsteuer", "wert": False, "zustand": "vorlaeufig",
                            "herkunft": {"herkunft": "vorjahr", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                            "schreiber": "import:vorjahr", "signal": {"signal_1": None, "signal_2": None}})
    assert st == 201, r
    st, erg = API.ergebnis(fid)
    assert st == 200, erg
    assert erg["grund"] == "gwg_tatbestand_offen", (
        f"ein vorlaeufiges 'nein' ist keine Antwort, grund={erg['grund']!r}")
