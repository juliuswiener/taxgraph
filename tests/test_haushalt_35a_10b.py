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
        runner.catala_p35a_haushaltsnahe({"minijob_aufwendungen": 0})
    except Exception:
        pytest.skip("Catala-Toolchain / pkg nicht verfügbar")
    return runner


# ---- § 35a: drei getrennte 20-%-Töpfe mit eigenem Deckel (510/4000/1200), additiv ----

@pytest.mark.parametrize("s,erwartet", [
    ({"minijob_aufwendungen": 2800}, 510),                                  # Abs. 1: 20 % = 560 > 510 -> Deckel
    ({"minijob_aufwendungen": 2000}, 400),                                  # Abs. 1: 20 % = 400 <= 510 -> ungekürzt
    ({"handwerker_arbeitskosten": 4500}, 900),                             # Abs. 3: 20 % = 900 <= 1200
    ({"handwerker_arbeitskosten": 10000}, 1200),                          # Abs. 3: 20 % = 2000 > 1200 -> Deckel
    ({"haushaltsnahe_dienstleistungen": 3000}, 600),                      # Abs. 2: 20 % = 600 <= 4000
    ({"minijob_aufwendungen": 2800, "handwerker_arbeitskosten": 10000}, 1710),  # 510 + 1200 (getrennte Deckel)
    ({}, 0),                                                              # kein Aufwand -> kein Phantom-Betrag
])
def test_p35a_seeds(R, s, erwartet):
    assert R.catala_p35a_haushaltsnahe(s) == erwartet


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
    p35a = R.catala_p35a_haushaltsnahe({"handwerker_arbeitskosten": 10000})   # 1200
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
