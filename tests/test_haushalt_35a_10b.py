"""§ 35a Haushaltsnahe + § 10b Spenden — Accessor-Einheitstests (charge29-Promotion, #7).

Prüft die zwei runner.py-Accessoren catala_p35a_haushaltsnahe / catala_p10b_spenden gegen die
charge29-Seeds (byte-gleich zu den module-Test-Scopes) UND die Wiring-Semantik über catala_gesamt:
§35a Roh → steuerermaessigungen (p32a floort auf verfügbare ESt), §10b Roh → sonderausgaben (additiv).
Toolchain-frei übersprungen (importorskip catala über runner)."""
import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "golden"))


@pytest.fixture(scope="module")
def R():
    try:
        import runner
        runner.catala_p35a_haushaltsnahe({"hh_in_eu_ewr": {"wert": True}})
    except Exception:
        pytest.skip("Catala-Toolchain / pkg nicht verfügbar")
    return runner


# ---- § 35a: drei getrennte 20-%-Töpfe mit eigenem Deckel (510/4000/1200), additiv, EURO ----

GATE = {"hh_in_eu_ewr": {"wert": True}, "hh_rechnung_unbar": {"wert": True}, "hh_handwerker_keine_foerderung": {"wert": True}}

@pytest.mark.parametrize("s,erwartet", [
    ({"hh_minijob_aufwendungen": 2800},  510),
    ({"hh_minijob_aufwendungen": 2000},  400),
    ({"hh_handwerker_arbeitskosten": 4500},  900),
    ({"hh_handwerker_arbeitskosten": 10000}, 1200),
    ({"hh_dienstleistungen": 3000},  600),
    ({"hh_minijob_aufwendungen": 2800, "hh_handwerker_arbeitskosten": 10000}, 1710),
    ({}, 0),
])
def test_p35a_seeds(R, s, erwartet):
    assert R.catala_p35a_haushaltsnahe({**GATE, **s}) == erwartet


def test_p35a_minijob_ohne_rechnung(R):
    """Minijob (Abs.1) braucht KEIN hh_rechnung_unbar — Abs.5 S.3 gilt nur Abs.2/3."""
    assert R.catala_p35a_haushaltsnahe(
        {"hh_in_eu_ewr": {"wert": True}, "hh_minijob_aufwendungen": 2800}) == 510


def test_p35a_handwerker_ohne_rechnung_null(R):
    """Handwerker (Abs.3) braucht hh_rechnung_unbar — ohne rechnung → Abs.2/3 nullt, Abs.1 bleibt."""
    assert R.catala_p35a_haushaltsnahe(
        {"hh_in_eu_ewr": {"wert": True}, "hh_handwerker_arbeitskosten": 4500}) == 0


def test_p35a_ohne_eu_ewr_null(R):
    """EU/EWR (Abs.4) gatet alle drei Absätze."""
    assert R.catala_p35a_haushaltsnahe(
        {"hh_handwerker_arbeitskosten": 4500, "hh_rechnung_unbar": {"wert": True}}) == 0


# ---- § 10b: min(Zuwendungen; 20 % GdE) ----

@pytest.mark.parametrize("s,erwartet", [
    ({"zuwendungen": 15000, "gesamtbetrag_der_einkuenfte": 50000}, 10000),  # 15000 > 20 % (10000) -> Deckel
    ({"zuwendungen": 5000, "gesamtbetrag_der_einkuenfte": 50000}, 5000),    # 5000 <= 10000 -> ungekürzt
    ({"zuwendungen": 10000, "gesamtbetrag_der_einkuenfte": 50000}, 10000),  # genau am Deckel
    ({"zuwendungen": 0, "gesamtbetrag_der_einkuenfte": 50000}, 0),          # keine Spende -> kein Abzug
])
def test_p10b_seeds(R, s, erwartet):
    assert R.catala_p10b_spenden(s) == erwartet


def test_35a_est_deckelung_floor(R):
    """§ 2 Abs. 6: §35a ist nicht erstattungsfähig. Übersteigt die Roh-Ermäßigung die verfügbare (tarifliche)
    ESt, floort p32a (wirksame_ermaessigung) die festzusetzende ESt auf 0 — nie negativ."""
    ns = R.catala_einkuenfte_nichtselbststaendig(
        {"veranlagungszeitraum": 2025, "bruttoarbeitslohn": 14000, "werbungskosten": 0})
    ohne = R.catala_gesamt({"veranlagungszeitraum": 2025, "veranlagung": "einzel",
                            "einkuenfte_nichtselbststaendig": ns})
    p35a = R.catala_p35a_haushaltsnahe({"hh_in_eu_ewr": {"wert": True}, "hh_rechnung_unbar": {"wert": True}, "hh_handwerker_arbeitskosten": 10000})   # 1200 EURO
    assert 0 < ohne < p35a          # Roh-Ermäßigung übersteigt die verfügbare ESt (Vorbedingung des Tests)
    mit = R.catala_gesamt({"veranlagungszeitraum": 2025, "veranlagung": "einzel",
                           "einkuenfte_nichtselbststaendig": ns, "steuerermaessigungen": p35a})
    assert mit == 0                 # gefloort, nicht ohne - p35a (= negativ)


def test_10b_speist_sonderausgaben_additiv(R):
    """§ 10b speist als Sonderausgabe § 2 Abs. 4: eine Spende senkt das zvE → niedrigere festzusetzende ESt
    als ohne Spende (bei identischem §19-Einkommen)."""
    ns = R.catala_einkuenfte_nichtselbststaendig(
        {"veranlagungszeitraum": 2025, "bruttoarbeitslohn": 60000, "werbungskosten": 0})
    p10b = R.catala_p10b_spenden({"zuwendungen": 3000, "gesamtbetrag_der_einkuenfte": ns})
    ohne = R.catala_gesamt({"veranlagungszeitraum": 2025, "veranlagung": "einzel",
                            "einkuenfte_nichtselbststaendig": ns})
    mit = R.catala_gesamt({"veranlagungszeitraum": 2025, "veranlagung": "einzel",
                           "einkuenfte_nichtselbststaendig": ns, "sonderausgaben": p10b})
    assert mit < ohne and p10b == 3000
