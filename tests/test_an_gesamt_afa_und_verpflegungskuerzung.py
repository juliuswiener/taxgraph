"""Fehlende Bausteine in der Scheibe `an_gesamt`.

FUND 1 (§ 7 Abs. 1 S. 4 EStG, AfA-Zwölftelung): `an_gesamt` fragt Anschaffungskosten und
Nutzungsdauer, aber nicht den Kaufmonat und nicht das Anschaffungsjahr-Flag. Der Zweig
rechnet trotzdem — mit dem vollen Jahresbetrag. Gemessen: 1.200 EUR / 3 Jahre = 400 EUR,
richtig im Kaufjahr (Oktober) sind 100 EUR. **300 EUR Abzug zu viel**, Richtung zu wenig
Steuer. Der Sperrgrund `arbeitsmittel_afa_ueber_gwg_offen` kann das nicht auffangen: er
liest `am_afa_ist_anschaffungsjahr`, und ein Feld, das die Scheibe nicht führt, kann nie
bestätigt werden — der Zweig ist für `an_gesamt` tot.

Die Tests laufen über `POST /fall` mit `"scheibe": "an_gesamt"` — der Weg, der laut
`api.py::event` und `tests/test_erreichbarkeit_gate.py` befahrbar ist. Dass die Scheibe
keine UI-Kachel hat, ändert daran nichts: „wird nicht angeboten" ist etwas anderes als
„ist nicht erreichbar".
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))

sys.path.insert(0, os.path.join(ROOT, "produkt", "store"))

import api as API        # noqa: E402
import server as SRV     # noqa: E402

sys.path.insert(0, HERE)
from test_paket_b_e2e_http import (   # noqa: E402
    AN_GESAMT_KEGEL, _an_gesamt_anlegen, _catala_da, _laie, _req, _val, base)

pytestmark = pytest.mark.usefixtures("base")


# Die zwei Felder, die `an_gesamt` nicht führt und die der AfA-Zweig trotzdem liest.
AFA_FELDER = ("am_anschaffung_monat", "am_afa_ist_anschaffungsjahr")

# § 9 Abs. 4a: `an_gesamt` führt die Tage-Trigger, aber nicht die Kürzung. Die fünf
# Eingabefelder sind askable, das sechste ist das BERECHNETE Ergebnis (askable: false,
# Kz E0205508) — es muss trotzdem in der Scheibe stehen, sonst filtert `_scheibe_bindung()`
# es aus der Deklaration und das XML zeigt die Kürzung nicht.
KUERZUNG_FELDER = ("vpf_fruehstuecke_gestellt_anzahl", "vpf_mittagessen_gestellt_anzahl",
                   "vpf_abendessen_gestellt_anzahl", "vpf_mahlzeiten_gezahltes_entgelt",
                   "vpf_steuerfreie_erstattung_betrag", "p9_4a_kuerzung_nach_entgelt")

# 100 volle 24h-Tage, 100 gestellte Frühstücke, kein Entgelt, keine Erstattung.
# § 9 Abs. 4a S. 8 Nr. 1 EStG: Frühstück kürzt um 20 % der 28-EUR-Pauschale = 5,60 EUR.
# 100 × 5,60 EUR = 560,00 EUR. Der Betrag stammt aus dem Gesetz, nicht aus einer Messung:
# sources/gesetze-im-internet/estg_p9_abs4a_2026-07-09.txt, S. 3 Nr. 1 ("28 Euro") und
# S. 8 Nr. 1 ("für Frühstück um 20 Prozent").
# EINHEIT: E0205508 steht in der Deklaration in EURO, nicht in Cent — der Ring liefert
# Cent, `_cent_nach_kz` schreibt daraus den Vordruckwert. Gemessen (nicht geraten):
# die erste Fassung dieses Tests erwartete 56_000 und bekam 560.
FRUEHSTUECKE = 100
KUERZUNG_EUR_EXACT = 560

# Beiwerk, damit die AfA überhaupt sichtbar wird: 12 × 1.000 EUR Übernachtung liegen über
# dem Arbeitnehmer-Pauschbetrag (1.230 EUR, § 9a S. 1 Nr. 1 Buchst. a). Ohne das lägen
# beide Fälle darunter, der Pauschbetrag griffe, und der Test wäre grün, ohne zu messen.
# Dieselbe Konstruktion wie tests/test_paket_b_e2e_http.py::test_gesamt_arbeitsmittel_afa_1200_3jahre.
UEBERNACHTUNG = [("uebernachtung_kosten_monat", 100000), ("uebernachtung_monate", 12),
                 ("uebernachtung_monate_bisher", 10), ("uebernachtung_im_inland", True),
                 ("uebernachtung_auswaerts", True), ("uebernachtung_alleinnutzung", True),
                 ("uebernachtung_keine_lange_unterbrechung", True)]


def _an_gesamt_kegel_mit_uebernachtung(bruttolohn: int = 5000000) -> list:
    """Der Standard-Kegel mit hohem Lohn und sichtbaren Übernachtungskosten.

    Hoher Lohn, damit die AfA-Differenz in der Progressionszone sichtbar wird und nicht im
    Grundfreibetrag verschwindet. Die Übernachtungswerte ERSETZEN die Nullwerte aus
    AN_GESAMT_KEGEL (gleiche feld_id, sonst HTTP 422 „hat schon ein aktives Event").
    """
    ersatz = dict(UEBERNACHTUNG)
    k = [(f, ersatz.get(f, bruttolohn if f == "bruttoarbeitslohn" else w))
         for f, w in AN_GESAMT_KEGEL]
    assert len({f for f, _ in k}) == len(k), "Kegel enthält doppelte feld_id"
    return k


def _an_gesamt_afa_fall(base, fid: str, ist_anschaffungsjahr: bool):
    """Ein an_gesamt-Fall mit 1.200 EUR Arbeitsmittel über 3 Jahre, Kauf im Oktober."""
    _an_gesamt_anlegen(base, fid, _an_gesamt_kegel_mit_uebernachtung())
    for feld, wert in (("am_anschaffungskosten", 120000), ("arbeitsmittel_nutzungsdauer", 3)):
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie(feld, wert))
        assert st == 201, f"{feld} nicht postbar: {st}"
    st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie("am_anschaffung_monat", 10))
    assert st == 201, f"am_anschaffung_monat nicht in an_gesamt postbar (HTTP {st})"
    st, _ = _req(base, "POST", f"/fall/{fid}/event",
                 _laie("am_afa_ist_anschaffungsjahr", ist_anschaffungsjahr))
    assert st == 201, f"am_afa_ist_anschaffungsjahr nicht in an_gesamt postbar (HTTP {st})"
    st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    _val("ergebnis", erg)
    return erg


# ---------------------------------------------------------------------------
# Fund 1: AfA-Zwölftelung (§ 7 Abs. 1 S. 4)
# ---------------------------------------------------------------------------

@pytest.mark.parametrize("feld_id", AFA_FELDER)
def test_an_gesamt_nimmt_die_afa_felder_an(base, feld_id):
    """Beide Felder sind über `POST /event` erreichbar — vorher HTTP 400.

    Rot auf HEAD: die Felder stehen in keiner der beiden Listen von `an_gesamt`
    (`felder` und `kegel`), `api.py::event` weist sie als „nicht in dieser Scheibe" ab.
    """
    _an_gesamt_anlegen(base, f"ag_afa_{feld_id}", _an_gesamt_kegel_mit_uebernachtung())
    wert = 10 if feld_id == "am_anschaffung_monat" else True
    st, _ = _req(base, "POST", f"/fall/ag_afa_{feld_id}/event", _laie(feld_id, wert))
    assert st == 201, (
        f"{feld_id} ist auf an_gesamt nicht postbar (HTTP {st}) — das Feld fehlt in "
        f"SCHEIBEN['an_gesamt']['felder']. Ohne es rechnet der AfA-Zweig den vollen "
        f"Jahresbetrag statt der Zwölftelung."
    )


def test_an_gesamt_fragt_den_kaufmonat(base):
    """Die beiden AfA-Felder erscheinen im Fragenkatalog von `an_gesamt`.

    Rot auf HEAD: sie fehlen in `felder`, also liefert `/fragen` sie nicht.
    """
    _an_gesamt_anlegen(base, "ag_afa_fragen", _an_gesamt_kegel_mit_uebernachtung())
    st, fr = _req(base, "GET", "/fall/ag_afa_fragen/fragen")
    assert st == 200
    ids = {q["feld_id"] for q in fr["fragen"]}
    fehlend = [f for f in AFA_FELDER if f not in ids]
    assert not fehlend, (
        f"an_gesamt fragt {fehlend} nicht — die Zwölftelung im Anschaffungsjahr "
        f"(§ 7 Abs. 1 S. 4) kann dort nicht auslösen."
    )


def test_an_gesamt_zwolftelung_rechnet_anteilig(base):
    """Der Euro-Wert: Kauf im Oktober → 3/12 von 400 EUR = 100 EUR AfA, nicht 400 EUR.

    Gemessen wird die festgesetzte Steuer, nicht `> 0`. Die Differenz Anschaffungsjahr gegen
    Folgejahr ist der Beleg: 300 EUR mehr Abzug im Folgejahr → dort weniger Steuer.

    Rot auf HEAD: der Kaufmonat ist nicht setzbar, der Zweig gibt für BEIDE Fälle 400 EUR
    zurück — die Differenz ist dann 0.
    """
    if not _catala_da():
        pytest.skip("Catala-Toolchain fehlt — der Euro-Wert ist nicht messbar")

    erg_aj = _an_gesamt_afa_fall(base, "ag_afa_aj", ist_anschaffungsjahr=True)
    erg_fj = _an_gesamt_afa_fall(base, "ag_afa_fj", ist_anschaffungsjahr=False)

    assert erg_aj["grund"] == "bestaetigt" and erg_fj["grund"] == "bestaetigt", \
        f"aj={erg_aj.get('grund')} fj={erg_fj.get('grund')}"

    steuern_aj = erg_aj["zahl_cent"]
    steuern_fj = erg_fj["zahl_cent"]
    assert steuern_aj is not None and steuern_fj is not None

    # 300 EUR mehr Werbungskosten im Folgejahr → 9100 Cent Steuerdifferenz.
    # NICHT der Wert aus der Schwester-Scheibe übernehmen: test_gesamt_arbeitsmittel_afa_1200_3jahre
    # misst 9300, aber das ist ein anderer Ring (gefaltete § 2-Gesamtsteuer) und damit ein
    # anderer Grenzsteuersatz. Kalibriert INNERHALB dieses Rings: zwei Folgejahr-Fälle, deren
    # Jahres-AfA sich um exakt 300 EUR unterscheidet (1.200/3 = 400 gegen 2.100/3 = 700),
    # ergeben 589100 − 580000 = 9100. Der kopierte Wert wäre eine Wette auf einen fremden Ring
    # gewesen; genau die Bauform von [[geltungsbereich-ungleich-verwendung-sieben-faelle]].
    AFA_AJ_FJ_DELTA_EXACT = 9100
    delta = steuern_aj - steuern_fj
    assert delta == AFA_AJ_FJ_DELTA_EXACT, (
        f"AfA-Differenzial Anschaffungsjahr/ Folgejahr delta={delta} ≠ {AFA_AJ_FJ_DELTA_EXACT}. "
        f"delta=0 heißt: der Kaufmonat kommt im Ring nicht an, es wird in beiden Fällen "
        f"der volle Jahresbetrag abgezogen (400 statt 100 EUR)."
    )


def test_an_gesamt_anschaffungsjahr_ohne_monat_sperrt(base):
    """Anschaffungsjahr bestätigt, Kaufmonat fehlt → Sperrgrund, keine stille Voll-AfA.

    Auf `gesamt` ist dieser Zweig seit dem Naht-Fix scharf. Auf `an_gesamt` war er tot:
    das Flag fehlte in der Scheibe, also konnte es nie bestätigt werden, und der Guard
    hielt das Fehlen für eine gültige Antwort („Folgejahr, volle AfA").
    """
    _an_gesamt_anlegen(base, "ag_afa_offen", _an_gesamt_kegel_mit_uebernachtung())
    for feld, wert in (("am_anschaffungskosten", 120000), ("arbeitsmittel_nutzungsdauer", 3),
                       ("am_afa_ist_anschaffungsjahr", True)):
        st, _ = _req(base, "POST", "/fall/ag_afa_offen/event", _laie(feld, wert))
        assert st == 201, f"{feld} nicht postbar (HTTP {st})"
    # am_anschaffung_monat bleibt unbeantwortet.

    st, erg = _req(base, "GET", "/fall/ag_afa_offen/ergebnis")
    _val("ergebnis", erg)
    assert erg["zahl_cent"] is None, (
        "Ohne Kaufmonat darf im Anschaffungsjahr keine Zahl herauskommen — sonst wird "
        "der volle Jahresbetrag stillschweigend abgezogen."
    )
    assert erg["grund"] == "arbeitsmittel_afa_ueber_gwg_offen", \
        f"grund={erg.get('grund')}"


def test_an_gesamt_folgejahr_ohne_monat_rechnet(base):
    """Kontrollfall: Anschaffungsjahr bestätigt FALSE → Folgejahr, voller Betrag, Monat egal.

    Ohne diesen Fall wäre der Sperrgrund oben nicht unterscheidbar von „sperrt immer".
    """
    if not _catala_da():
        pytest.skip("Catala-Toolchain fehlt")
    _an_gesamt_anlegen(base, "ag_afa_fj_ohne_monat", _an_gesamt_kegel_mit_uebernachtung())
    for feld, wert in (("am_anschaffungskosten", 120000), ("arbeitsmittel_nutzungsdauer", 3),
                       ("am_afa_ist_anschaffungsjahr", False)):
        st, _ = _req(base, "POST", "/fall/ag_afa_fj_ohne_monat/event", _laie(feld, wert))
        assert st == 201, f"{feld} nicht postbar (HTTP {st})"
    st, erg = _req(base, "GET", "/fall/ag_afa_fj_ohne_monat/ergebnis")
    _val("ergebnis", erg)
    assert erg["grund"] == "bestaetigt" and erg["zahl_cent"] is not None, \
        f"Folgejahr ohne Monat muss rechnen, bekam grund={erg.get('grund')}"
# ---------------------------------------------------------------------------
# Fund 2: Mahlzeitenkürzung (§ 9 Abs. 4a)
# ---------------------------------------------------------------------------

@pytest.mark.parametrize("feld_id", KUERZUNG_FELDER)
def test_an_gesamt_nimmt_die_kuerzungsfelder_an(base, feld_id):
    """Die Kürzungs-Felder sind über `POST /event` erreichbar — vorher HTTP 400.

    `an_gesamt` führt `VERPFLEGUNG_TAGE`, aber nicht `VERPFLEGUNG_KUERZUNG`. Der Nutzer
    kann „ja, N Mahlzeiten wurden gestellt" deshalb nicht ehrlich eingeben.
    """
    _an_gesamt_anlegen(base, f"ag_vpf_{feld_id}", _an_gesamt_kegel_mit_uebernachtung())
    wert = 5 if "anzahl" in feld_id else 0
    st, _ = _req(base, "POST", f"/fall/ag_vpf_{feld_id}/event", _laie(feld_id, wert))
    assert st == 201, (
        f"{feld_id} ist auf an_gesamt nicht postbar (HTTP {st}) — das Feld fehlt in "
        f"SCHEIBEN['an_gesamt']['felder']. Die Mahlzeitenkürzung nach § 9 Abs. 4a "
        f"kann dort nicht angegeben werden."
    )


def test_an_gesamt_verpflegungskuerzung_senkt_die_zahl(base):
    """100 Frühstücke gestellt → weniger Werbungskosten → höhere Steuer.

    Gemessen wird die Steuer selbst, nicht nur `vollstaendig`: der Ring kürzt, das
    Ergebnis muss die Kürzung zeigen. Ohne Mahlzeiten und mit Mahlzeiten sind zwei
    Fälle, deren Differenz die Kürzung sichtbar macht.

    Rot auf HEAD: die Anzahl-Felder sind nicht postbar (HTTP 400), der Fall kommt gar
    nicht erst zustande.
    """
    if not _catala_da():
        pytest.skip("Catala-Toolchain fehlt — der Euro-Wert ist nicht messbar")

    def _vpf_fall(fid: str, fruehstuecke: int):
        # 100 Tage à 24h: die Kürzung muss über dem Arbeitnehmer-Pauschbetrag sichtbar
        # bleiben und darf nicht vom Günstigerprinzip verdeckt werden.
        # vpf_monate_am_ort=1 (≤ 3) hält die Dreimonatsfrist aus dem Weg: die ganze
        # Pauschale bleibt innerhalb der Frist, sonst kürzt die Frist die Bezugsgröße.
        kegel = [(f, (100 if f == "tage_24h" else w))
                 for f, w in _an_gesamt_kegel_mit_uebernachtung()]
        kegel = [(f, (True if f == "vpf_keine_mahlzeitengestellung" else w)) for f, w in kegel]
        _an_gesamt_anlegen(base, fid, kegel)
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _laie("vpf_monate_am_ort", 1))
        assert st == 201
        for feld in ("vpf_fruehstuecke_gestellt_anzahl", "vpf_mittagessen_gestellt_anzahl",
                     "vpf_abendessen_gestellt_anzahl"):
            st, _ = _req(base, "POST", f"/fall/{fid}/event",
                         _laie(feld, fruehstuecke if feld.endswith("fruehstuecke_gestellt_anzahl") else 0))
            assert st == 201, f"{feld} nicht postbar (HTTP {st})"
        st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
        _val("ergebnis", erg)
        st, dek = _req(base, "GET", f"/fall/{fid}/deklaration")
        assert st == 200, f"deklaration nicht abrufbar (HTTP {st}): {dek}"
        return erg, dek

    ohne, dek_ohne = _vpf_fall("ag_vpf_ohne", 0)
    mit, dek_mit = _vpf_fall("ag_vpf_mit", FRUEHSTUECKE)

    assert ohne["grund"] == "bestaetigt" and mit["grund"] == "bestaetigt", \
        f"ohne={ohne.get('grund')} mit={mit.get('grund')}"
    assert mit["zahl_cent"] > ohne["zahl_cent"], (
        f"100 gestellte Frühstücke müssen die Steuer erhöhen (Kürzung senkt die "
        f"Werbungskosten). ohne={ohne['zahl_cent']} mit={mit['zahl_cent']} — keine "
        f"oder die falsche Richtung."
    )

    # Der Wert selbst, nicht nur die Richtung. Das Ergebnis der Kürzung steht als
    # E0205508 in der Deklaration — dasselbe Feld, das die Scheibe ohne die Kürzung
    # aus der Bindung gefiltert hätte. Ohne Mahlzeiten gibt es keine Kürzung.
    assert "E0205508" not in dek_ohne.get("deklaration", {}), dek_ohne.get("deklaration")
    assert dek_mit["deklaration"].get("E0205508") == KUERZUNG_EUR_EXACT, (
        f"E0205508 = {dek_mit['deklaration'].get('E0205508')!r}, erwartet "
        f"{KUERZUNG_EUR_EXACT} EUR (100 × 20 % × 28 EUR nach § 9 Abs. 4a S. 8 Nr. 1 EStG). "
        f"Die Scheibe hat den Ring-Wert nicht deklariert."
    )
    # Die Deklaration ist vollständig — der Fall kommt ohne Sperre durch.
    assert dek_mit["eingaben_konsistent"] is True, dek_mit
