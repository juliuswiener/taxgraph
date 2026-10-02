"""Die Catala-Regel § 35a rechnet nie mit (Vault tickets/haushaltsnahe-catala-regel-wird-nie-gerechnet.md,
gemessen 2026-10-02, Bericht ~/.cache/taxgraph-tmp/berichte/messung-35a-bereich.md).

`runner.HN` (rules/estg/p35a/haushaltsnahe.catala_en) wird importiert und nie aufgerufen; die Zahl kommt
aus der Python-Nachbildung `runner.catala_p35a_haushaltsnahe`. Beide weichen ab, sobald ein Topf unter
seinem Deckel liegt und kein Vielfaches von 5 Euro ist: die Nachbildung rundet je Topf auf ganze Euro ab,
die Regel rechnet centgenau. Die Seeds in test_haushalt_35a_10b.py sind alle Vielfache von 5 — dort
sieht man es nicht.

Verlangt ist nur, was in jeder Lösung gilt: bei offenen Toren (EU/EWR, Rechnung unbar, keine Förderung,
keine Mitveranlagung) liefert der Accessor die abgerundete Euro-Zahl der Regel. Die Kontrolle mit
Vielfachen von 5 MUSS grün bleiben: sie belegt, dass der Test die Rundung misst, nicht den Aufruf."""
import pytest

TORE = {"hh_in_eu_ewr": {"wert": True}, "hh_rechnung_unbar": {"wert": True}}


@pytest.fixture(scope="module")
def R():
    try:
        import runner
    except Exception:
        pytest.skip("Catala-Toolchain / pkg nicht verfügbar")
    return runner


def _beide(R, minijob, dienstleistungen, handwerker):
    regel = R.HN.haushaltsnahe(R.HN.HaushaltsnaheIn(
        minijob_aufwendungen_in=R.Money(f"{minijob}.00"),
        haushaltsnahe_dienstleistungen_in=R.Money(f"{dienstleistungen}.00"),
        handwerker_arbeitskosten_in=R.Money(f"{handwerker}.00"))).steuerermaessigung
    accessor = R.catala_p35a_haushaltsnahe({"hh_minijob_aufwendungen": minijob, "hh_dienstleistungen":
                                            dienstleistungen, "hh_handwerker_arbeitskosten": handwerker, **TORE})
    return int(regel) // 100, accessor


@pytest.mark.parametrize("toepfe", [(2800, 3000, 10000), (2550, 20000, 6000), (5, 10, 15)])
def test_kontrolle_vielfache_von_5_stimmen_ueberein(R, toepfe):
    regel, accessor = _beide(R, *toepfe)
    assert accessor == regel


@pytest.mark.parametrize("toepfe,regel_euro", [((4, 4, 4), 2), ((0, 1, 5999), 1200)])
def test_accessor_rechnet_wie_die_catala_regel(R, toepfe, regel_euro):
    regel, accessor = _beide(R, *toepfe)
    assert regel == regel_euro                      # Vorbedingung: die Regel selbst wie gemessen
    assert accessor == regel, f"{toepfe} EUR: Regel {regel} EUR, Accessor {accessor} EUR"


@pytest.mark.parametrize("toepfe,erwartet", [((0, 0, -500), 0), ((-500, 0, 0), 0), ((-1000, 3000, 5999), 1799)])
def test_negativer_topf_rechnet_wie_vorher_null(R, toepfe, erwartet):
    """Die Regel allein ergäbe −100 € je −500 € (gemessen 2026-10-02); bis negativer-aufwand-umgeht-
    pflichtfrage gebaut ist, zählt ein negativer Topf 0 € wie vor der Umstellung."""
    regel, accessor = _beide(R, *toepfe)
    assert regel < erwartet                         # Vorbedingung: die Regel allein rechnet schlechter
    assert accessor == erwartet
