"""BACKLOG stille-null-klasse-c (Variante b): eine VORLAEUFIGE Instanz einer der 6 Klasse-C-
Aggregat-Funktionen (gwg, kind ×3, p23) faellt still aus der Summe — die Zahl ist korrekt
(nur_bestaetigt-Filter greift), aber /ergebnis meldet es nirgends (grund bleibt "bestaetigt",
offen bleibt []). Bericht: reports/adjudikation/instanz_stille_null_2026-08-07.md.

Fix: ergebnis() sammelt die Basis-Feld-IDs aller vorlaeufigen Klasse-C-Instanzen in "offen".
KEINE Sperre — grund bleibt "bestaetigt", zahl_cent bleibt die (korrekte) gefilterte Zahl.

4 Gruppen getestet (gwg, kind_kv_pv, schulgeld, p23) — je 1 Test mit 3 Assertions:
1. vorlaeufig: Feld-ID in offen, zahl_cent == Referenz (60k-Basisfall, 1392400).
2. bestaetigt: Feld-ID NICHT in offen, zahl_cent != Referenz (Kontrolle — beweist, dass das
   Feld bei Bestaetigung tatsaechlich wirkt, sonst waere "nicht in offen" durch Wirkungslosigkeit
   vorgetaeuscht statt durch echtes Fix-Verhalten).

Referenzzahlen aus dem Bericht (60k-Basisfall wie test_p23_ueber_ring_accessor):
  Referenz (kein Klasse-C-Feld gesetzt): 1392400
  schulgeld=300000 bestaetigt: 1359200
  kind_kv=100000/kind_pv=50000 bestaetigt: 1336300
  kind_behinderten_pb (GdB 80) bestaetigt: 1311400
"""
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "golden"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api as API        # noqa: E402
import audit              # noqa: E402

REFERENZ = 1392400

_KEGEL = [
    ("veranlagung", "einzel"), ("bruttoarbeitslohn", 6000000),
    ("vv_einnahmen", 0), ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0),
    ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 100),
    ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("basis_kv", 0), ("basis_pv", 0),
    ("versicherungsart", "gesetzlich_an"), ("vorsorge_arbeitslosenversicherung", 0),
    ("vorsorge_erwerbsunfaehigkeit", 0), ("vorsorge_unfall_haftpflicht", 0),
    ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0),
    ("mit_anspruch_auf_zuschuss", False),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
]


def _laie(fld, w, zustand="bestaetigt"):
    signal2 = f"ok@{fld}" if zustand == "bestaetigt" else None
    return {"feld_id": fld, "wert": w, "zustand": zustand,
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie",
            "signal": {"signal_1": None, "signal_2": signal2}}


def _neuer_fall(tmp_path, monkeypatch, fid, kegel_overrides=None):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    st, r = API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fid})
    assert st == 201, r
    overrides = kegel_overrides or {}
    for feld, wert in _KEGEL:
        st, r = API.event(fid, _laie(feld, overrides.get(feld, wert)))
        assert st == 201, f"{feld}={wert}: {st} {r}"
    return fid


class TestGwgStilleNull:
    def test_vorlaeufig_taucht_in_offen_auf(self, tmp_path, monkeypatch):
        fid = _neuer_fall(tmp_path, monkeypatch, "sn-gwg-v")
        st, r = API.event(fid, _laie("gwg_anschaffungskosten_netto", 50000, "vorlaeufig"))
        assert st == 201, r
        st, erg = API.ergebnis(fid)
        assert st == 200, erg
        assert erg["grund"] == "bestaetigt", erg
        assert erg["zahl_cent"] == REFERENZ, (
            f"stille Null muss die Zahl unveraendert lassen: {erg['zahl_cent']} statt {REFERENZ}")
        assert "gwg_anschaffungskosten_netto" in erg["offen"], (
            f"vorlaeufige gwg-Instanz muss in offen auftauchen, war: {erg['offen']}")

    def test_bestaetigt_nicht_in_offen_und_zahl_aendert_sich(self, tmp_path, monkeypatch):
        # kein_gewinn=True widerspricht einem bestaetigten gwg (flag_widersprueche) — hier geht es
        # nur um die stille-Null-Meldung, nicht um den Flag-Konsistenz-Check, also Flag im Kegel
        # gleich als False setzen statt es nachträglich zu überschreiben.
        fid = _neuer_fall(tmp_path, monkeypatch, "sn-gwg-b", kegel_overrides={"kein_gewinn": False})
        st, r = API.event(fid, _laie("gwg_anschaffungskosten_netto", 50000, "bestaetigt"))
        assert st == 201, r
        st, erg = API.ergebnis(fid)
        assert st == 200, erg
        assert erg["grund"] == "bestaetigt", erg
        assert erg["zahl_cent"] != REFERENZ, (
            "Kontrolle: bestaetigtes gwg muss die Zahl bewegen, sonst beweist "
            "'nicht in offen' nur Wirkungslosigkeit statt echtes Fix-Verhalten")
        assert "gwg_anschaffungskosten_netto" not in erg["offen"], erg["offen"]


class TestKindKvPvStilleNull:
    def test_vorlaeufig_taucht_in_offen_auf(self, tmp_path, monkeypatch):
        fid = _neuer_fall(tmp_path, monkeypatch, "sn-kvpv-v")
        st, r = API.event(fid, _laie("kind_idnr", "99988877766", "vorlaeufig"))
        assert st == 201, r
        st, r = API.event(fid, _laie("kind_kv", 100000, "vorlaeufig"))
        assert st == 201, r
        st, r = API.event(fid, _laie("kind_pv", 50000, "vorlaeufig"))
        assert st == 201, r
        st, erg = API.ergebnis(fid)
        assert st == 200, erg
        assert erg["grund"] == "bestaetigt", erg
        assert erg["zahl_cent"] == REFERENZ, (
            f"stille Null muss die Zahl unveraendert lassen: {erg['zahl_cent']} statt {REFERENZ}")
        assert "kind_kv" in erg["offen"] or "kind_pv" in erg["offen"], (
            f"vorlaeufige kind_kv/kind_pv-Instanz muss in offen auftauchen, war: {erg['offen']}")

    def test_bestaetigt_nicht_in_offen_und_zahl_aendert_sich(self, tmp_path, monkeypatch):
        fid = _neuer_fall(tmp_path, monkeypatch, "sn-kvpv-b")
        st, r = API.event(fid, _laie("kind_idnr", "99988877766", "bestaetigt"))
        assert st == 201, r
        st, r = API.event(fid, _laie("kind_kv", 100000, "bestaetigt"))
        assert st == 201, r
        st, r = API.event(fid, _laie("kind_pv", 50000, "bestaetigt"))
        assert st == 201, r
        st, erg = API.ergebnis(fid)
        assert st == 200, erg
        assert erg["grund"] == "bestaetigt", erg
        assert erg["zahl_cent"] == 1336300, (
            f"bestaetigtes kind_kv/kind_pv soll 1336300 ergeben, war {erg['zahl_cent']}")
        assert "kind_kv" not in erg["offen"] and "kind_pv" not in erg["offen"], erg["offen"]


class TestSchulgeldStilleNull:
    def test_vorlaeufig_taucht_in_offen_auf(self, tmp_path, monkeypatch):
        fid = _neuer_fall(tmp_path, monkeypatch, "sn-schulgeld-v")
        st, r = API.event(fid, _laie("schulgeld", 300000, "vorlaeufig"))
        assert st == 201, r
        st, erg = API.ergebnis(fid)
        assert st == 200, erg
        assert erg["grund"] == "bestaetigt", erg
        assert erg["zahl_cent"] == REFERENZ, (
            f"stille Null muss die Zahl unveraendert lassen: {erg['zahl_cent']} statt {REFERENZ}")
        assert "schulgeld" in erg["offen"], (
            f"vorlaeufige schulgeld-Instanz muss in offen auftauchen, war: {erg['offen']}")

    def test_bestaetigt_nicht_in_offen_und_zahl_aendert_sich(self, tmp_path, monkeypatch):
        fid = _neuer_fall(tmp_path, monkeypatch, "sn-schulgeld-b")
        st, r = API.event(fid, _laie("schulgeld", 300000, "bestaetigt"))
        assert st == 201, r
        st, erg = API.ergebnis(fid)
        assert st == 200, erg
        assert erg["grund"] == "bestaetigt", erg
        assert erg["zahl_cent"] == 1359200, (
            f"bestaetigtes schulgeld soll 1359200 ergeben, war {erg['zahl_cent']}")
        assert "schulgeld" not in erg["offen"], erg["offen"]


class TestP23StilleNull:
    def test_vorlaeufig_taucht_in_offen_auf(self, tmp_path, monkeypatch):
        fid = _neuer_fall(tmp_path, monkeypatch, "sn-p23-v")
        for feld, wert in [
            ("p23_veraeusserungspreis", 20000000),
            ("p23_anschaffung_herstellungskosten", 15000000),
            ("p23_werbungskosten", 500000),
            ("p23_veraeusserungs_typ", "grundstueck"),
        ]:
            st, r = API.event(fid, _laie(feld, wert, "vorlaeufig"))
            assert st == 201, f"{feld}={wert}: {st} {r}"
        st, erg = API.ergebnis(fid)
        assert st == 200, erg
        assert erg["grund"] == "bestaetigt", erg
        assert erg["zahl_cent"] == REFERENZ, (
            f"stille Null muss die Zahl unveraendert lassen: {erg['zahl_cent']} statt {REFERENZ}")
        assert "p23_veraeusserungspreis" in erg["offen"], (
            f"vorlaeufige p23-Instanz muss in offen auftauchen, war: {erg['offen']}")

    def test_bestaetigt_nicht_in_offen_und_zahl_aendert_sich(self, tmp_path, monkeypatch):
        fid = _neuer_fall(tmp_path, monkeypatch, "sn-p23-b")
        for feld, wert in [
            ("p23_veraeusserungspreis", 20000000),
            ("p23_anschaffung_herstellungskosten", 15000000),
            ("p23_werbungskosten", 500000),
            ("p23_veraeusserungs_typ", "grundstueck"),
        ]:
            st, r = API.event(fid, _laie(feld, wert, "bestaetigt"))
            assert st == 201, f"{feld}={wert}: {st} {r}"
        st, erg = API.ergebnis(fid)
        assert st == 200, erg
        assert erg["grund"] == "bestaetigt", erg
        assert erg["zahl_cent"] == 3265600, (
            f"bestaetigtes p23 soll 3265600 ergeben (bekannter Referenzwert), war {erg['zahl_cent']}")
        assert "p23_veraeusserungspreis" not in erg["offen"], erg["offen"]

    def test_gemischte_instanz_nur_das_vorlaeufige_feld_in_offen(self, tmp_path, monkeypatch):
        """Eine Instanz mit gemischtem Zustand (3 Felder bestaetigt, 1 vorlaeufig) — meet_zustand
        macht die GANZE Instanz vorlaeufig, aber offen darf NUR das tatsaechlich unbestaetigte
        Feld listen, nicht die 3 bereits bestaetigten (sonst meldet die API Felder als offen,
        die der Nutzer laengst bestaetigt hat)."""
        fid = _neuer_fall(tmp_path, monkeypatch, "sn-p23-mix")
        for feld, wert in [
            ("p23_veraeusserungspreis", 20000000),
            ("p23_anschaffung_herstellungskosten", 15000000),
            ("p23_veraeusserungs_typ", "grundstueck"),
        ]:
            st, r = API.event(fid, _laie(feld, wert, "bestaetigt"))
            assert st == 201, f"{feld}={wert}: {st} {r}"
        st, r = API.event(fid, _laie("p23_werbungskosten", 500000, "vorlaeufig"))
        assert st == 201, r
        st, erg = API.ergebnis(fid)
        assert st == 200, erg
        assert erg["grund"] == "bestaetigt", erg
        assert erg["zahl_cent"] == REFERENZ, (
            f"stille Null (Instanz-meet=vorlaeufig) muss die Zahl unveraendert lassen: "
            f"{erg['zahl_cent']} statt {REFERENZ}")
        assert "p23_werbungskosten" in erg["offen"], erg["offen"]
        assert "p23_veraeusserungspreis" not in erg["offen"], (
            f"bestaetigtes Feld darf nicht als offen gemeldet werden: {erg['offen']}")
        assert "p23_anschaffung_herstellungskosten" not in erg["offen"], erg["offen"]
        assert "p23_veraeusserungs_typ" not in erg["offen"], erg["offen"]
