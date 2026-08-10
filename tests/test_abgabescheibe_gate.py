"""Gates gegen zwei Nähte, die am 2026-08-10 zusammen gefunden wurden.

Beide entstehen an derselben Stelle: eine Scheibe ist eine TEILMENGE, und was in ihr fehlt,
fällt still heraus statt laut aufzufallen.

**Naht 1 — Abzugs-Kz ohne sein Kürzungs-Kz.** Auf `gesamt` waren die drei Verpflegungs-Tage-Kz
(E0205409/E0205302/E0205201) gebunden, das berechnete Kürzungsfeld `p9_4a_kuerzung_nach_entgelt`
(E0205508) aber nicht: es stand in keinem der Feld-Tupel in api_constants.py, und `gesamt` baut
seine Feldliste aus Tupeln. `n_vor_gwg` war korrekt — die Scheibe zieht ihre Felder über
`felder_datei` aus der YAML, wo das Feld gebunden ist. Der Ring kürzte also (gemessen: 8400 Cent),
die Deklaration zeigte nur die vollen Tage. **Die Erklärung wäre zu hoch ausgefallen — under-tax,
Haftung beim Nutzer.** Das ist die Abgabe-Scheibe für den Arbeitnehmerfall.

**Naht 2 — Abgabe auf einer Scheibe ohne Stammdaten.** `/einreichen` akzeptierte jede Scheibe.
Auf `an_gesamt` (0 von 12 Stammdatenfeldern im Kegel) lief der Fall bis checkESt und scheiterte
dort an fehlenden Stammdaten, die der Nutzer dort strukturell nie hätte liefern können — POST
/event lehnt sie mit 400 "nicht in dieser Scheibe" ab. Fail-closed war es, aber die Meldung zeigte
auf den Nutzer statt auf die Scheibenwahl.
"""

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/import", "produkt/store"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API              # noqa: E402
import api_constants as AC     # noqa: E402
import audit                   # noqa: E402
import store as ST             # noqa: E402


@pytest.fixture(autouse=True)
def _isoliert(tmp_path, monkeypatch):
    """Fälle in tmp_path statt ins echte produkt/haut/faelle/.

    Ohne das kollidieren Wiederholungsläufe mit ihren eigenen Altfällen
    ("Fall existiert bereits") — was wie ein Testfehler aussieht, aber keiner ist.
    """
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))


def _abgabefaehige_scheiben():
    """Scheiben, auf denen eine Erklärung entstehen kann: alle Stammdaten im Kegel."""
    return [n for n, cfg in API.SCHEIBEN.items()
            if all(f in set(cfg.get("felder") or ()) for f in AC.STAMMDATEN_FELDER)]


def _bindung(scheibe, fid):
    API.fall_anlegen({"fall_id": fid, "scheibe": scheibe, "veranlagungszeitraum": 2025})
    return API._scheibe_bindung(API.lade_fall(fid))


def _kz(bindung):
    return {v.get("elster_kz") for v in bindung.values() if isinstance(v, dict)}


# --------------------------------------------------------------- Naht 1: Kürzung

# Abzugs-Kz -> das Kz, das den Abzug wieder mindert. Fehlt der Wert rechts, während links
# gebunden ist, deklariert die Erklärung einen ungeminderten Abzug.
KUERZUNGS_PAARE = [
    (("E0205409", "E0205302", "E0205201"), "E0205508",
     "§ 9 Abs. 4a S. 8 EStG — Verpflegungspauschale gekürzt um gestellte Mahlzeiten"),
]


def test_abgabescheibe_hat_zu_jedem_abzug_seine_kuerzung():
    """Wo ein Abzugs-Kz deklariert wird, MUSS sein Kürzungs-Kz mitdeklarierbar sein.

    Das ist der Gate gegen Naht 1. Er prüft die Bindung, nicht den Ring: der Ring rechnete
    schon vorher richtig, nur die Deklaration verschwieg das Ergebnis.
    """
    scheiben = _abgabefaehige_scheiben()
    assert scheiben, "keine abgabefähige Scheibe gefunden — Testvoraussetzung kaputt"

    for scheibe in scheiben:
        kz = _kz(_bindung(scheibe, f"gate_kuerz_{scheibe}"))
        for abzugs_kz, kuerz_kz, norm in KUERZUNGS_PAARE:
            gebundene_abzuege = [k for k in abzugs_kz if k in kz]
            if not gebundene_abzuege:
                continue  # Scheibe kennt diesen Abzug gar nicht — konsistent
            assert kuerz_kz in kz, (
                f"Scheibe {scheibe!r} deklariert {gebundene_abzuege} ohne {kuerz_kz}. "
                f"{norm}. Der Ring kürzt, die Erklärung verschweigt es → sie fällt ZU HOCH "
                f"aus (under-tax, Haftung beim Nutzer).")


def test_kuerzungsbetrag_landet_in_der_deklaration_auf_gesamt():
    """Scharf: Ring rechnet die Kürzung UND sie steht als E0205508 in der Deklaration.

    Der vorige Test prüft die Bindung. Dieser prüft, dass der berechnete Wert auch ankommt —
    eine Bindung ohne Wert wäre eine stille Null.
    """
    fid = "gate_kuerz_wert"
    API.fall_anlegen({"fall_id": fid, "scheibe": "gesamt", "veranlagungszeitraum": 2025})
    store = API.lade_fall(fid)
    for feld, wert in [("tage_24h", 10),
                       ("vpf_fruehstuecke_gestellt_anzahl", 5),
                       ("vpf_mittagessen_gestellt_anzahl", 5)]:
        ST.append_event(store, feld_id=feld, wert=wert, zustand="bestaetigt",
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft",
                                  "haftung": "nutzer"},
                        schreiber="laie",
                        signal={"signal_1": {"typ": "laie_eingabe"},
                                "signal_2": "laie_bestaetigt"})
    API.speichere_fall(fid, store)

    felder, _ = ST.materialisiere(API.lade_fall(fid))
    felder = API._mit_ring_werten(felder, 2025)
    eintrag = felder.get("p9_4a_kuerzung_nach_entgelt")
    assert eintrag, "Ring liefert p9_4a_kuerzung_nach_entgelt nicht — Vorbedingung kaputt"
    assert eintrag["wert"] > 0, (
        f"Ring kürzt nicht bei 5 Frühstücken + 5 Mittagessen: {eintrag['wert']}")

    # und das Feld ist auf dieser Scheibe auch deklarierbar
    kz = _kz(API._scheibe_bindung(API.lade_fall(fid)))
    assert "E0205508" in kz, (
        f"Ring kürzt {eintrag['wert']} Cent, aber E0205508 ist auf 'gesamt' nicht gebunden — "
        f"genau die stille Differenz, die die Erklärung zu hoch macht.")


# ------------------------------------------------------- Naht 2: Scheibe ohne Stammdaten

def test_einreichen_lehnt_scheibe_ohne_stammdaten_ab():
    """Eine Teilrechnungs-Scheibe darf nicht in die Abgabe laufen — und muss das SAGEN."""
    unfaehig = [n for n, cfg in API.SCHEIBEN.items()
                if any(f not in set(cfg.get("felder") or ()) for f in AC.STAMMDATEN_FELDER)]
    assert unfaehig, "keine Teilrechnungs-Scheibe vorhanden — Testvoraussetzung kaputt"

    for scheibe in unfaehig:
        fid = f"gate_scheibe_{scheibe}"
        API.fall_anlegen({"fall_id": fid, "scheibe": scheibe, "veranlagungszeitraum": 2025})
        st, resp = API.einreichen(fid, {})
        assert st == 409, f"{scheibe!r}: erwartet 409, kam {st} — {resp}"
        assert resp["grund"] == "scheibe_nicht_abgabefaehig", (
            f"{scheibe!r}: Grund zeigt nicht auf die Scheibenwahl: {resp.get('grund')!r}")
        assert resp["eingereicht"] is False
        # Die Meldung muss die Scheibe nennen, nicht den Nutzer beschuldigen
        assert scheibe in resp["hinweis"], f"{scheibe!r}: Hinweis nennt die Scheibe nicht"
        assert resp["fehlende_stammdatenfelder"], (
            f"{scheibe!r}: 409 ohne Angabe, welche Felder fehlen")


def test_abgabefaehige_scheibe_wird_nicht_von_der_scheibenpruefung_gesperrt():
    """Gegenprobe: der neue Check darf den Normalfall nicht blockieren.

    Ohne diesen Test wäre ein Check, der ALLES ablehnt, grün — die Standard-Falle bei
    fail-closed-Gates.
    """
    for scheibe in _abgabefaehige_scheiben():
        fid = f"gate_ok_{scheibe}"
        API.fall_anlegen({"fall_id": fid, "scheibe": scheibe, "veranlagungszeitraum": 2025})
        st, resp = API.einreichen(fid, {})
        assert resp.get("grund") != "scheibe_nicht_abgabefaehig", (
            f"{scheibe!r} ist abgabefähig, wird aber von der Scheibenprüfung gesperrt: {resp}")
