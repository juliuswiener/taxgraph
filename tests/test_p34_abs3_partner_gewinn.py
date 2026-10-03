"""§ 34 Abs. 3 für Person A, Veräußerungsgewinn beim Ehegatten: sperren statt falsch rechnen.

Vault backlog/taxgraph/p34-antrag-ohne-kennzahl-erreicht-elster-nicht.md (AK2b), Entscheid main 2026-10-03:
Trigger = ROHER Partner-Gewinn > 0 (nicht netto_vg_partner), „gesperrt statt falsch gerechnet“.

Gemessen am 2026-10-03 auf 649d61a9 (zusammen, A berechtigt mit 500.000 EUR, Partner 300.000 EUR, beide
Freibetrag-Fragen bestätigt): `/ergebnis` lieferte `bestaetigt` und 24.194.600 ct, ohne Sperre. Der Chooser
(bescheid_zweige) nimmt Abs. 3 für A, der Partner-Gewinn bleibt ungeglättet im Rest (§ 34 Abs. 3 S. 3:
„vorbehaltlich des Absatzes 1“ — die Fünftelung auf den Partner-Gewinn fehlt). Die Entscheidung von Julius
(p34-fuenftelung-umfasst-beide-ehegatten): beide Ehegatten werden geglättet. Bis das gebaut ist, sperrt der
Guard mit `abs3_partner_gewinn_offen`.

Der Guard sperrt genau dann, wenn der Chooser Abs. 3 nimmt UND der Partner einen Gewinn trägt:
  veranlagung = zusammen ∧ Antrag wahr ∧ `_abs3_eligible` ∧ 0 < netto_vg(A) <= 5 Mio ∧ Partner-VG > 0.
Nur BESTÄTIGTE Felder urteilen (wie der Chooser mit nur_bestaetigt=True; ein vorläufiger Wert ist kein
Beleg, partner_check zählt ebenso). Ein Betrag <= 0 sperrt nie.

Abweichungsfall roh/netto: Partner-VG 40.000 EUR liegt unter dem Freibetrag (netto_vg_partner 0). Der rohe
Trigger sperrt, der Netto-Trigger sperrte nicht. Gewollt (main): zu weite Sperre vor Zahl ohne Sperre.

NULL LLM."""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/eingang", "produkt/mapping", "produkt/bescheid", "golden",
             "elster/submission"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api as API  # noqa: E402
import api_constants as AC  # noqa: E402

from _kegel import partner_kegel_fuer  # noqa: E402
from test_p34_antrag_kennzahl import _fall, _laie  # noqa: E402

GRUND = "abs3_partner_gewinn_offen"
VG_A = 50_000_000            # 500.000 EUR, über dem Freibetrag, unter 5 Mio
VG_PARTNER = 30_000_000      # 300.000 EUR, Freibetrag des Partners 0


def _bauen(tmp_path, monkeypatch, fid, scheibe="gesamt", antrag=True, partner_vg=VG_PARTNER, **gesetzt):
    """Zusammenveranlagung, A mit Antrag; der Partner trägt `partner_vg` (None = Feld fehlt)."""
    _fall(tmp_path, monkeypatch, fid, scheibe=scheibe, veranlagung="zusammen",
          antrag_ermaessigter_satz=antrag, **gesetzt)
    ereignisse = [(f, w) for f, w in partner_kegel_fuer() if f in AC.SCHEIBEN[scheibe]["felder"]]
    if partner_vg is not None:
        ereignisse += [("rentner_veraeusserungsgewinn_partner", partner_vg),
                       ("rentner_alter_55_oder_berufsunfaehig_partner", True),
                       ("rentner_freibetrag_erstmalig_partner", True)]
    for feld, wert in ereignisse:
        st, r = API.event(fid, _laie(feld, wert))
        assert st == 201, (feld, wert, st, r)


def _ergebnis(fid):
    st, e = API.ergebnis(fid)
    assert st == 200, e
    return e["grund"], e["zahl_cent"]


# -------------------------------------------------------------------- über die API, mit dem gemessenen Fall

@pytest.mark.parametrize("scheibe", ["gesamt", "rentner_gesamt"])
def test_abs3_fuer_a_und_gewinn_beim_partner_sperrt_statt_zu_rechnen(tmp_path, monkeypatch, scheibe):
    """Der gemessene Fall: vorher `("bestaetigt", 24194600)` (gesamt), jetzt Sperre ohne Zahl, auch in /deklaration."""
    _bauen(tmp_path, monkeypatch, "s", scheibe=scheibe)
    assert _ergebnis("s") == (GRUND, None)
    st, d = API.deklaration("s")
    assert st == 409 and d["grund"] == GRUND, (st, d.get("grund"))
    st, e = API.ergebnis("s")
    assert e["klartext"] and "Ehepartner" in e["klartext"], e.get("klartext")


@pytest.mark.parametrize("name, partner_vg, antrag", [
    ("ohne_partner_gewinn", 0, True),
    ("partner_gewinn_fehlt", None, True),
    ("ohne_antrag", VG_PARTNER, False),
])
def test_kontrollen_ohne_sperre_rechnen_weiter(tmp_path, monkeypatch, name, partner_vg, antrag):
    """Gegenproben: A Abs. 3 ohne Partner-Gewinn, und Partner-Gewinn ohne Antrag (Abs. 1 glättet beide) rechnen."""
    _bauen(tmp_path, monkeypatch, "k", partner_vg=partner_vg, antrag=antrag)
    grund, zahl = _ergebnis("k")
    assert grund == "bestaetigt" and isinstance(zahl, int) and zahl > 0, (name, grund, zahl)


# -------------------------------------------------------------------- die Bibliothek: jede Bedingung einzeln

def _snap(**abweichend):
    """Snapshot eines berechtigten Falls: zusammen, A Abs. 3, Partner-VG 300.000 EUR; (wert, bestaetigt) je Feld."""
    felder = {"veranlagung": ("zusammen", True), "antrag_ermaessigter_satz": (True, True),
              "geburtsjahr": (1960, True), "dauernd_berufsunfaehig": (False, True),
              "ermaessigung_einmal_genutzt": (False, True),
              "rentner_alter_55_oder_berufsunfaehig": (True, True), "rentner_freibetrag_erstmalig": (True, True),
              "rentner_veraeusserungsgewinn": (VG_A, True), "rentner_veraeusserungs_betriebsart": ("gewerbe", True),
              "rentner_veraeusserungsgewinn_partner": (VG_PARTNER, True),
              "rentner_alter_55_oder_berufsunfaehig_partner": (True, True),
              "rentner_freibetrag_erstmalig_partner": (True, True),
              # ohne diese zwei Gruppen sperrt der Guard früher: flag_konsistenz_offen (Gewinn ohne kein_gewinn =
              # nein), partner_kegel_offen (zusammen ohne die Pflichtfelder von Person B)
              "kein_gewinn": (False, True), **{f: (w, True) for f, w in partner_kegel_fuer()}}
    felder.update(abweichend)
    return {k: {"wert": w, "zustand": "bestaetigt" if b else "vorlaeufig"} for k, (w, b) in felder.items()
            if w is not None}


def _grund(scheibe, **abweichend):
    # Bindung der Scheibe, nicht die volle: sonst spricht `flag_widersprueche` auf Felder an, die der Fall nicht führt
    return API._an_gesamt_sperrgrund(_snap(**abweichend), AC.SCHEIBEN[scheibe], 2025, None,
                                     API._scheibe_bindung({"scheibe": scheibe}))


@pytest.mark.parametrize("scheibe", ["gesamt", "rentner_gesamt"])
def test_die_bibliothek_sperrt_den_vollen_fall(scheibe):
    assert _grund(scheibe) == GRUND


@pytest.mark.parametrize("name, abw", [
    # Betrag: nur ein Gewinn > 0 sperrt
    ("partner_vg_null", {"rentner_veraeusserungsgewinn_partner": (0, True)}),
    ("partner_vg_negativ", {"rentner_veraeusserungsgewinn_partner": (-1, True)}),
    ("partner_vg_fehlt", {"rentner_veraeusserungsgewinn_partner": (None, True)}),
    # nur bestätigte Felder urteilen: ein vorläufiger Wert ist kein Beleg
    ("partner_vg_vorlaeufig", {"rentner_veraeusserungsgewinn_partner": (VG_PARTNER, False)}),
    ("antrag_vorlaeufig_ja", {"antrag_ermaessigter_satz": (True, False)}),
    ("antrag_vorlaeufig_nein", {"antrag_ermaessigter_satz": (False, False)}),
    ("veranlagung_vorlaeufig", {"veranlagung": ("zusammen", False)}),
    ("geburtsjahr_vorlaeufig", {"geburtsjahr": (1960, False)}),
    ("gewinn_a_vorlaeufig", {"rentner_veraeusserungsgewinn": (VG_A, False)}),
    # der Chooser nimmt Abs. 3 nicht: keine falsche Zahl, also keine Sperre
    ("antrag_nein", {"antrag_ermaessigter_satz": (False, True)}),
    ("antrag_fehlt", {"antrag_ermaessigter_satz": (None, True)}),
    ("einzelveranlagung", {"veranlagung": ("einzel", True)}),
    ("a_zu_jung", {"geburtsjahr": (1990, True)}),
    ("a_schon_genutzt", {"ermaessigung_einmal_genutzt": (True, True)}),
    ("a_netto_null", {"rentner_veraeusserungsgewinn": (4_000_000, True)}),     # 40.000 EUR < Freibetrag
    ("a_kein_gewinn", {"rentner_veraeusserungsgewinn": (None, True)}),
], ids=lambda x: x if isinstance(x, str) else "")
@pytest.mark.parametrize("scheibe", ["gesamt", "rentner_gesamt"])
def test_die_bibliothek_sperrt_nicht_ohne_eine_der_bedingungen(scheibe, name, abw):
    assert _grund(scheibe, **abw) is None, name


@pytest.mark.parametrize("partner_vg, sperrt", [
    (1, True),                  # 1 ct: roher Betrag > 0
    (4_000_000, True),          # 40.000 EUR: unter dem Freibetrag, netto 0 -> der rohe Trigger sperrt (Abweichungsfall)
    (VG_PARTNER, True),
])
def test_der_rohe_partner_gewinn_entscheidet_nicht_der_netto_gewinn(partner_vg, sperrt):
    assert (_grund("gesamt", rentner_veraeusserungsgewinn_partner=(partner_vg, True)) == GRUND) == sperrt


def test_ueber_fuenf_millionen_behaelt_den_eigenen_grund():
    """Die 5-Mio-Sperre steht vor dieser: ihr Text trifft den Fall (A über der Grenze) genauer."""
    assert _grund("gesamt", rentner_veraeusserungsgewinn=(600_000_000, True)) == "abs3_ueber_5mio_offen"


def test_der_grund_steht_in_klartext_und_schema():
    import json
    import bescheid_deklaration as BD
    assert BD.SPERRGRUND_KLARTEXT[GRUND].strip()
    schema = json.load(open(os.path.join(ROOT, "produkt", "haut", "api_schema", "ergebnis.json"), encoding="utf-8"))
    assert GRUND in json.dumps(schema)
