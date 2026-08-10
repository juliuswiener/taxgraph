"""Gate: § 33b Abs. 1 S. 1 Wahlrecht Partner (Stufe 2b-Partner, rentner_gesamt zusammen).

Anlass 2026-08-10, BACKLOG p33b-partner-pb-doppelabzug: der Partner-Pauschbetrag lief
unconditional additiv neben agb_aufwendungen -- 1.168-1.234 EUR stiller Doppelabzug bei
Zusammenveranlagung mit Partner-GdB>=20 (oder hilflos/blind/taubblind) UND behinderungs-
bedingten Aufwendungen des Partners, ohne dass je ein Wahlrecht (§ 33b Abs. 1 S. 1/2)
geprüft wurde. Fix spiegelt die bestehende Person-A-Mechanik (Stufe 2b) unabhängig auf den
Partner: eigenständige Felder behinderungsbedingte_aufwendungen_partner (davon-Teilmenge,
fallweit wie agb_aufwendungen) + behinderungsbedingte_aufwendungen_wahlrecht_pb_partner
(askable bool), eigenständiger Sperrgrund behinderungsbedingte_aufwendungen_wahlrecht_
partner_offen in _an_gesamt_sperrgrund, eigenständige Kürzung in _shared_steuer_sonder_agb
(NICHT elif-gekettet an Person A -- beide Wahlrechte sind unabhängig, jede Kombination
ist möglich).

Statute-Achse (§ 33b Abs. 1 S. 1/2 EStG, sources/gesetze-im-internet/estg_p33b_2026-07-13.txt):
Abs. 1 S. 1 gilt PRO PERSON (Subjekt "Menschen mit Behinderungen", Anspruch nach Abs. 2/3
individuell nach eigenem GdB). Abs. 1 S. 2 "einheitlich" bindet die Wahl EINER Person über
ihre eigenen Aufwandsarten (Hilfe/Pflege/Wäsche) hinweg, NICHT zwei Ehegatten aneinander --
kein Textbefund für eine Kopplung. Daher: unabhängige Wahlrechte für Person A und Partner.
"""

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/import", "produkt/store"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API              # noqa: E402
import audit                   # noqa: E402


@pytest.fixture(autouse=True)
def _isoliert(tmp_path, monkeypatch):
    """Fälle in tmp_path statt ins echte produkt/haut/faelle/ -- sonst kollidieren Wiederholungsläufe."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))


def _laie(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


_BASIS_ZUSAMMEN = [
    ("rentner_renten_art", "gesetzliche_rente"), ("rentner_jahresrente", 6000000),
    ("rentner_renten_beginn_jahr", 2025), ("rentner_alter_bei_rentenbeginn", 0),
    ("rentner_grad_der_behinderung", 0), ("rentner_hilflos_blind_taubblind", False),
    ("rentner_pflegegrad", 0), ("rentner_gepflegter_hilflos", False),
    ("rentner_hinterbliebenenbezuege", False), ("veranlagung", "zusammen"),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("versicherungsart", "gesetzlich_an"), ("basis_kv", 0), ("basis_pv", 0),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
]

# Partner: GdB 100 (>=20 -> eigener PB) + 3.000 EUR behinderungsbedingte Aufwendungen des
# Partners. agb_aufwendungen == derselbe Betrag, sonst waere die agB-Seite der Kuerzung bei
# Wahlrecht=True unsichtbar (agb_cent liefe sofort auf 0, egal was abgezogen wird) -- dieselbe
# Fallgrube wie bei Person A (api.py-Kommentar zur Bauanleitungs-Zahlenprobe).
_PARTNER_GDB100 = [("rentner_grad_der_behinderung_partner", 100),
                    ("rentner_hilflos_blind_taubblind_partner", False)]
_PARTNER_AGB = [("behinderungsbedingte_aufwendungen_partner", 300000), ("agb_aufwendungen", 300000)]


def _anlegen_und_fuellen(fall_id, zusatz_felder):
    st, _ = API.fall_anlegen({"fall_id": fall_id, "scheibe": "rentner_gesamt", "veranlagungszeitraum": 2025})
    assert st == 201
    for fid, wert in _BASIS_ZUSAMMEN + zusatz_felder:
        st, resp = API.event(fall_id, _laie(fid, wert))
        assert st == 201, f"{fid}: {st} {resp}"
    return API.ergebnis(fall_id)


# ---- Gegenprobe (PFLICHT laut Auftrag): Paar OHNE Partner-GdB bleibt unberührt ----

def test_ohne_partner_gdb_unveraendert():
    """Kein Partner-GdB, keine Partner-Aufwendungen -- der Normalfall (Gate-Polarität,
    519199e-Präzedenz): darf NICHT sperren, muss regulär durchlaufen."""
    st, erg = _anlegen_und_fuellen("ohne_gdb", [])
    assert st == 200
    assert erg["grund"] == "bestaetigt"
    assert isinstance(erg["zahl_cent"], int)


# ---- Sperrgrund: Partner-GdB + Partner-agB vorhanden, Wahlrecht unbeantwortet ----

def test_partner_wahlrecht_unbeantwortet_sperrt():
    st, erg = _anlegen_und_fuellen("partner_offen", _PARTNER_GDB100 + _PARTNER_AGB)
    assert st == 200, "darf nicht crashen -- muss als reguläres Ergebnis mit Sperrgrund zurückkommen"
    assert erg["grund"] == "behinderungsbedingte_aufwendungen_wahlrecht_partner_offen", (
        f"unbeantwortetes Partner-Wahlrecht muss sperren, grund={erg.get('grund')!r}")
    assert erg["zahl_cent"] is None


def test_partner_gdb_ohne_agb_sperrt_nicht():
    """Gegenprobe (Gate-Polarität): Partner-GdB vorhanden, aber KEINE behinderungsbedingten
    Aufwendungen des Partners -- kein Wahlrecht zu treffen, darf nicht sperren."""
    st, erg = _anlegen_und_fuellen("gdb_ohne_agb", _PARTNER_GDB100)
    assert erg["grund"] == "bestaetigt", f"Partner-GdB ohne Partner-agB darf nicht sperren: {erg}"
    assert isinstance(erg["zahl_cent"], int)


def test_partner_agb_ohne_gdb_sperrt_nicht():
    """Gegenprobe (Gate-Polarität): Partner-agB-Betrag vorhanden, aber KEIN Partner-GdB (kein
    eigener PB) -- kein Wahlrecht möglich, darf nicht sperren."""
    st, erg = _anlegen_und_fuellen("agb_ohne_gdb", _PARTNER_AGB)
    assert erg["grund"] == "bestaetigt", f"Partner-agB ohne Partner-GdB darf nicht sperren: {erg}"
    assert isinstance(erg["zahl_cent"], int)


# ---- Wahlrecht=True vs. False: müssen unterschiedliche Ergebnisse liefern ----

def test_partner_wahlrecht_true_und_false_liefern_unterschiedliche_ergebnisse():
    _, erg_true = _anlegen_und_fuellen("partner_true", _PARTNER_GDB100 + _PARTNER_AGB
                                        + [("behinderungsbedingte_aufwendungen_wahlrecht_pb_partner", True)])
    assert erg_true["grund"] == "bestaetigt"
    _, erg_false = _anlegen_und_fuellen("partner_false", _PARTNER_GDB100 + _PARTNER_AGB
                                         + [("behinderungsbedingte_aufwendungen_wahlrecht_pb_partner", False)])
    assert erg_false["grund"] == "bestaetigt"
    assert erg_true["zahl_cent"] != erg_false["zahl_cent"], (
        f"Wahlrecht=True/False müssen sich unterscheiden (sonst ist die Kürzung ein no-op): "
        f"true={erg_true['zahl_cent']} false={erg_false['zahl_cent']}")


def test_partner_wahlrecht_false_landet_auf_der_gdb_losen_baseline():
    """Der starke Anker: bei Wahlrecht=False MUSS dieselbe Zahl herauskommen wie ohne Partner-GdB.

    Ergaenzt 2026-08-10 nach einer Mutationsprobe, die der Differenzierungstest oben NICHT fing:
    wird der True-Zweig der Kuerzung deaktiviert (`if False:` statt der Bedingung), unterscheiden
    sich True und False weiterhin — nur eben falsch (True traegt dann PB UND ungekuerzte agB,
    also genau den Doppelabzug, den dieser Fix beseitigt). "Unterschiedlich" ist damit zu schwach
    als Kriterium; es beweist, dass die Kuerzung kein no-op ist, nicht dass sie stimmt.

    Diese Invariante ist enger und stammt aus dem Person-A-Bau (p33b Stufe 2b): wer den
    Pauschbetrag NICHT nimmt, steht exakt wie jemand ohne GdB — der PB ist der einzige Effekt des
    GdB, und genau der entfaellt. Gemessen: Baseline 566.000 ct == Wahlrecht=False 566.000 ct,
    waehrend True bei 508.600 ct liegt.
    """
    _, ohne_gdb = _anlegen_und_fuellen("anker_ohne_gdb", _PARTNER_AGB)
    _, false_ = _anlegen_und_fuellen("anker_false", _PARTNER_GDB100 + _PARTNER_AGB
                                     + [("behinderungsbedingte_aufwendungen_wahlrecht_pb_partner", False)])
    assert false_["zahl_cent"] == ohne_gdb["zahl_cent"], (
        f"Wahlrecht=False muss bit-identisch zur GdB-losen Baseline sein: "
        f"false={false_['zahl_cent']} baseline={ohne_gdb['zahl_cent']}. Weicht es ab, wird der "
        f"Partner-Pauschbetrag entweder nicht entfernt (Doppelabzug bleibt) oder es wird zuviel "
        f"abgezogen.")


def test_partner_wahlrecht_true_kuerzt_die_agb_wirklich():
    """Gegenstueck: bei Wahlrecht=True MUSS die agB-Kuerzung sichtbar wirken.

    Verglichen wird derselbe Fall mit und ohne `behinderungsbedingte_aufwendungen_partner`. Wirkt
    die Kuerzung, ist das Ergebnis MIT der Teilmenge hoeher (weniger Abzug). Ohne diesen Test
    bleibt ein toter True-Zweig unbemerkt — genau die Mutation, die der Differenzierungstest
    durchgehen liess.
    """
    _, mit_bb = _anlegen_und_fuellen("kuerz_mit", _PARTNER_GDB100 + _PARTNER_AGB
                                     + [("behinderungsbedingte_aufwendungen_wahlrecht_pb_partner", True)])
    _, ohne_bb = _anlegen_und_fuellen("kuerz_ohne",
                                      _PARTNER_GDB100 + [("agb_aufwendungen", 300000)]
                                      + [("behinderungsbedingte_aufwendungen_partner", 0),
                                         ("behinderungsbedingte_aufwendungen_wahlrecht_pb_partner", True)])
    assert mit_bb["zahl_cent"] > ohne_bb["zahl_cent"], (
        f"Mit behinderungsbedingten Aufwendungen des Partners muss die agB gekuerzt werden, die "
        f"Steuer also HOEHER liegen: mit={mit_bb['zahl_cent']} ohne={ohne_bb['zahl_cent']}. "
        f"Gleichstand heisst, der True-Zweig kuerzt nicht — Pauschbetrag und volle agB stuenden "
        f"nebeneinander (§ 33b Abs. 1 S. 1 'anstelle').")


# ---- Unabhängigkeit von Person A ----

def test_partner_wahlrecht_unabhaengig_von_person_a():
    """Person A hat in _BASIS_ZUSAMMEN keinen eigenen GdB -- Person As Wahlrecht-Frage bleibt
    unberührt (behinderungsbedingte_aufwendungen_wahlrecht_offen darf NICHT feuern), während
    der Partner-Sperrgrund unabhängig greift."""
    st, erg = _anlegen_und_fuellen("nur_partner_offen", _PARTNER_GDB100 + _PARTNER_AGB)
    assert erg["grund"] == "behinderungsbedingte_aufwendungen_wahlrecht_partner_offen"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
