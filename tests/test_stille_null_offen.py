"""BACKLOG stille-null-klasse-c (Variante b): eine VORLAEUFIGE Instanz einer der 3 Klasse-C-
Aggregat-Funktionen (gwg, kind, p23_veraeusserung) faellt still aus der Summe — die Zahl ist
korrekt (nur_bestaetigt-Filter greift), aber /ergebnis meldet es nirgends (grund bleibt
"bestaetigt", offen bleibt []). Bericht: reports/adjudikation/instanz_stille_null_2026-08-07.md.

Fix: ergebnis() sammelt die Basis-Feld-IDs aller vorlaeufigen Klasse-C-Instanzen in "offen"
(produkt/haut/api.py::_ergebnis_roh, offen_c-Sammlung). KEINE Sperre — grund bleibt "bestaetigt",
zahl_cent bleibt die (korrekte) gefilterte Zahl.

Je 1 Test pro Gruppe fuer den vorlaeufig-Fall (Feld-ID in offen, zahl_cent == Referenz), plus den
bestaetigt-Fall als Kontrolle, wo erreichbar (Feld-ID NICHT in offen, zahl_cent != Referenz —
beweist, dass das Feld bei Bestaetigung tatsaechlich wirkt, sonst waere "nicht in offen" durch
Wirkungslosigkeit vorgetaeuscht statt durch echtes Fix-Verhalten):
  gwg (TestGwgStilleNull), kind_kv_pv (TestKindKvPvStilleNull), schulgeld
  (TestSchulgeldStilleNull): beide Faelle.
  p23_veraeusserung (TestP23StilleNull): NUR der vorlaeufig-Fall ist erreichbar. Der bestaetigt-
  Fall ist fuer diese Gruppe kategorisch gesperrt: kein_p23_verkauf=False (noetig, damit ein
  bestaetigter §23-Betrag nicht dem Screening-Flag "keine privaten Verkaeufe" widerspricht,
  FC.flag_widersprueche) liegt in fremd_arten fuer die Scheibe "gesamt" (api_constants.py, seit
  714b40e/2026-08-31) und sperrt JEDEN bestaetigten §23-Verkauf mit
  grund="einkunftsart_nicht_ring_faehig", BEVOR _ergebnis_roh die offen_c-Sammlung ueberhaupt
  erreicht (der Sperrgrund kommt aus bescheid_deklaration.py, vor der offen_c-Zeile in api.py).
  Absichtlich, Produktentscheidung Julius 2026-08-31 (test_p23_accessor.py::
  test_p23_ueber_ring_accessor). test_bestaetigter_verkauf_bleibt_gesperrt_nicht_stille_null
  haelt das fest, statt eine stille-Null-Behandlung zu erwarten, die es fuer diese Gruppe nicht
  geben kann.

Referenzzahlen aus dem Bericht (60k-Basisfall wie test_p23_ueber_ring_accessor):
  Referenz (kein Klasse-C-Feld gesetzt): 1392400
  schulgeld=300000 bestaetigt: 1359200
  kind_kv=100000/kind_pv=50000 bestaetigt: 1336300
  kind_idnr+kind_kv=100000 bestaetigt, schulgeld=300000 vorlaeufig: 1392400 (die ganze
  "kind"-Instanz gilt als nicht vollstaendig bestaetigt und traegt nichts zur Summe bei — dieselbe
  Instanz-Semantik wie beim ehemaligen p23-Mischfall)

Die Eigenschaft "gemischte Instanz (einzelne Felder bestaetigt, eines vorlaeufig) — offen listet
nur das tatsaechlich unbestaetigte Feld, nicht die ganze Instanz" ist NICHT gruppenspezifisch
(offen_c sammelt pro FELD, nicht pro Instanz). Sie braucht deshalb keine p23-Instanz und wird von
TestGemischteKindInstanzStilleNull an der "kind"-Gruppe geprueft (kind_idnr/kind_kv bestaetigt,
schulgeld vorlaeufig — alle drei tragen instanz_gruppe: kind in den Bindungstabellen, keins davon
ist gesperrt); empirisch verifiziert vor dem Einbau (Wegwerf-Skript, nicht Teil des Repos).

Repariert 2026-09-07 (Instructor-Auftrag "der kaputte Fix von gestern"): die vorherige Fassung
dieser Datei (Commit 7da4a07, 2026-09-06) hatte fuer TestP23StilleNull zwei Faelle mit
`kegel_overrides={"kein_p23_verkauf": False}` versehen, um flag_widersprueche zu umgehen — und
damit ungewollt den fremd_arten-Guard aus 714b40e (2026-08-31) ausgeloest. Behauptet war "per
Mutationsprobe bestaetigt", tatsaechlich lief der Fix nie gegen diesen Guard-Konflikt (beide Faelle
standen rot: grund="einkunftsart_nicht_ring_faehig" statt "bestaetigt"). Vorfrage vor der
Reparatur: ist der fremd_arten-Guard hier im Recht? Ja — s. oben, belegt durch
test_p23_accessor.py und die SCHEIBEN-Konfiguration in api_constants.py (kein_p23_verkauf steht
in KEINER Scheibe ausserhalb von fremd_arten). Also wurden die zwei betroffenen Tests umgebaut,
nicht der Guard. Gegenprobe (fremd_arten-Zweig in bescheid_deklaration.py bzw. die "kind"-Gruppe
in der offen_c-Sammlung probeweise entfernt): beide umgebauten Tests wurden wieder rot, danach
zurueckgesetzt — s. Bericht an den Instructor.
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
    ("kein_p23_verkauf", True),
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
        # Schritt 2 (2026-09-07, bescheid_deklaration.py): die drei GWG-Tatbestandsfragen sind
        # CONDITIONAL-MANDATORY, sobald die Instanz einen Betrag > 0 traegt -- unabhaengig vom
        # Bestaetigungsstatus des Betrags selbst (analog _hh_instanz_positiv). Ohne sie sperrt
        # gwg_tatbestand_offen, bevor die stille-Null-Sammlung unten ueberhaupt erreicht wird.
        for _fld in ("gwg_bewegliches_selbstaendig_nutzbar", "gwg_netto_ohne_vorsteuer", "gwg_verzeichnis_ab_250"):
            st, r = API.event(fid, _laie(_fld, True))
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
        # Schritt 2 (2026-09-07): dieselben drei Pflichtfragen wie oben, hier bestaetigt statt
        # vorlaeufig -- sonst sperrt gwg_tatbestand_offen VOR der hier zu pruefenden Zahl-Wirkung.
        for _fld in ("gwg_bewegliches_selbstaendig_nutzbar", "gwg_netto_ohne_vorsteuer", "gwg_verzeichnis_ab_250"):
            st, r = API.event(fid, _laie(_fld, True))
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

    def test_bestaetigter_verkauf_bleibt_gesperrt_nicht_stille_null(self, tmp_path, monkeypatch):
        """Anders als bei gwg/kind/schulgeld erreicht ein BESTAETIGTER §23-Verkauf die stille-
        Null-Sammlung (offen_c) nie: kein_p23_verkauf=False (noetig, sonst widerspricht der
        bestaetigte Betrag dem Screening-Flag "keine privaten Verkaeufe", flag_widersprueche)
        liegt in fremd_arten fuer die Scheibe "gesamt" (api_constants.py, seit 714b40e/2026-08-31)
        und sperrt VORHER mit grund="einkunftsart_nicht_ring_faehig" — absichtlich, keine
        Scheibe kann einen bestaetigten §23-Verkauf tatsaechlich rechnen (Produktentscheidung
        Julius 2026-08-31, s. test_p23_accessor.py::test_p23_ueber_ring_accessor). Diese Fassung
        (2026-09-07) haelt genau das fest; die vorherige Fassung erwartete faelschlich dasselbe
        Verhalten wie bei gwg/kind/schulgeld (grund bleibt "bestaetigt") und stand deshalb rot."""
        fid = _neuer_fall(tmp_path, monkeypatch, "sn-p23-b", kegel_overrides={"kein_p23_verkauf": False})
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
        assert erg["grund"] == "einkunftsart_nicht_ring_faehig", erg
        assert erg["zahl_cent"] is None, erg


class TestGemischteKindInstanzStilleNull:
    def test_gemischte_instanz_nur_das_vorlaeufige_feld_in_offen(self, tmp_path, monkeypatch):
        """Eine Instanz mit gemischtem Zustand (2 Felder bestaetigt, 1 vorlaeufig) — meet_zustand
        macht die GANZE Instanz vorlaeufig (traegt nichts zur Summe bei), aber offen darf NUR das
        tatsaechlich unbestaetigte Feld listen, nicht die bereits bestaetigten (sonst meldet die
        API Felder als offen, die der Nutzer laengst bestaetigt hat). Gruppe "kind" statt (wie
        urspruenglich) "p23_veraeusserung": die Eigenschaft ist nicht p23-spezifisch (offen_c
        sammelt pro Feld, nicht pro Instanz), und ein bestaetigter p23-Verkauf ist kategorisch
        gesperrt (s. TestP23StilleNull.test_bestaetigter_verkauf_bleibt_gesperrt_nicht_stille_null)
        — kind_idnr/kind_kv/schulgeld tragen alle drei instanz_gruppe: kind und sind ungesperrt."""
        fid = _neuer_fall(tmp_path, monkeypatch, "sn-kind-mix")
        for feld, wert in [("kind_idnr", "99988877766"), ("kind_kv", 100000)]:
            st, r = API.event(fid, _laie(feld, wert, "bestaetigt"))
            assert st == 201, f"{feld}={wert}: {st} {r}"
        st, r = API.event(fid, _laie("schulgeld", 300000, "vorlaeufig"))
        assert st == 201, r
        st, erg = API.ergebnis(fid)
        assert st == 200, erg
        assert erg["grund"] == "bestaetigt", erg
        assert erg["zahl_cent"] == REFERENZ, (
            f"stille Null (Instanz-meet=vorlaeufig) muss die Zahl unveraendert lassen: "
            f"{erg['zahl_cent']} statt {REFERENZ}")
        assert "schulgeld" in erg["offen"], erg["offen"]
        assert "kind_idnr" not in erg["offen"], (
            f"bestaetigtes Feld darf nicht als offen gemeldet werden: {erg['offen']}")
        assert "kind_kv" not in erg["offen"], erg["offen"]
