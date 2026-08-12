"""K1 Zwei-Signal-Sicherheits-Goldens S1-S8 (dev-2, 2026-07-20).

Der Kern-Sicherheits-Invariant der UI-Runde: ein LLM/externer/Beleg/Kontoauszug-VORSCHLAG bewegt die
Steuer-Summe NIE ohne menschliches signal_2 — und darf ein human-only-Feld (Wahlrecht/Status/Identität/
ABWESENHEITS-Erklärung/Allokation) gar nicht erst VORSCHLAGEN (Feld-Katalog). STRUKTURELL bewiesen an jeder
Ebene: Store (append_event), Mapping (deklariere = die Steuer-Summen-Eingabe), Traverser (relevanz = die
Frage-Sichtbarkeit). KEIN echter LLM-Call — nur konstruierte Events/Fixtures (Julius' Cap NUR für den Live-Call,
getestet in test_haut_chat.py, dev-1). Ergänzt die Store-Guards (test_store) um den Katalog + den e2e-Invariant.
"""
from __future__ import annotations
import glob
import os
import sys

import pytest
yaml = pytest.importorskip("yaml")

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/store", "produkt/mapping", "produkt/traverser", "produkt/haut",
             "oracle/gettsim/_catala/pkg", "golden"):
    sys.path.insert(0, os.path.join(ROOT, _sub))
import store as ST          # noqa: E402
import est_mapping as EM    # noqa: E402
import traverser as TR      # noqa: E402

TS = "2026-07-20T12:00:00+00:00"


def _bindung() -> dict:
    b = {}
    for f in glob.glob(os.path.join(ROOT, "produkt", "bindung", "bindung_*.yaml")):
        for e in (yaml.safe_load(open(f, encoding="utf-8")).get("bindungen") or []):
            b[e["feld_id"]] = e
    return b


BINDUNG = _bindung()
KATALOG = ST.lade_katalog(BINDUNG)


def _leer():
    return ST.leerer_store(2025, fall_id="k1-sicherheit")


def _vorschlag(store, feld, wert, schreiber, herkunft_name, *, katalog=KATALOG):
    """Ein VORLÄUFIGES Vorschlags-Event (signal_2=null) über den Store-Choke-Point."""
    return ST.append_event(store, feld_id=feld, wert=wert, zustand="vorlaeufig",
                           herkunft={"herkunft": herkunft_name, "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                           schreiber=schreiber, signal={"signal_1": {"typ": "fixture"}, "signal_2": None},
                           katalog=katalog, ts=TS)


# ---------- S1: ein Vorschlag ist NICHT in der Steuer-Summen-Eingabe (Mapping) ----------
def test_s1_vorschlag_nicht_in_deklaration():
    """llm:chat schreibt agb_aufwendungen VORLÄUFIG → est_mapping.deklariere() führt es in `unvollstaendig`,
    NICHT in `deklaration` → die Steuer-Bemessung (die NUR die deklaration liest) ignoriert es."""
    s = _leer()
    _vorschlag(s, "agb_aufwendungen", 120000, "llm:chat", "llm_vorschlag")
    felder, _ = ST.materialisiere(s)
    erg = EM.deklariere(felder, BINDUNG)
    assert "agb_aufwendungen" in {u["feld_id"] for u in erg["unvollstaendig"]}
    assert erg["vollstaendig"] is False


# ---------- S2: nach menschlichem Confirm (signal_2) fließt es in die Deklaration ----------
def test_s2_bestaetigt_fliesst_in_deklaration():
    """Der Mensch bestätigt (zustand=bestaetigt, signal_2 gesetzt, ersetzt=Vorschlag) → deklariere führt
    agb_aufwendungen NICHT mehr als unvollständig; der Wert ist jetzt eine gültige Steuer-Eingabe."""
    s = _leer()
    ev = _vorschlag(s, "agb_aufwendungen", 120000, "llm:chat", "llm_vorschlag")
    ST.append_event(s, feld_id="agb_aufwendungen", wert=120000, zustand="bestaetigt",
                    herkunft={"herkunft": "nutzer", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                    schreiber="ui:laie", signal={"signal_1": None, "signal_2": "confirm@agb"},
                    ersetzt=ev["event_id"], ts=TS)
    felder, _ = ST.materialisiere(s)
    erg = EM.deklariere(felder, BINDUNG)
    assert "agb_aufwendungen" not in {u["feld_id"] for u in erg["unvollstaendig"]}


# ---------- S3/S4: kein Vorschlags-Schreiber kann direkt bestätigen (Store fail-closed A) ----------
@pytest.mark.parametrize("schreiber,herkunft", [
    ("llm:chat", "llm_vorschlag"),
    ("berechnet:maps", "berechnet"),
    ("import:kontoauszug", "kontoauszug"),
    ("import:beleg", "beleg_import"),
])
def test_s3_s4_vorschlagsschreiber_kann_nicht_bestaetigen(schreiber, herkunft):
    """Jeder Vorschlags-Schreiber, der zustand=bestaetigt + signal_2≠null versucht → ValueError (Auflage A,
    VOR dem Katalog-Check). Die KI/der Dienst kann strukturell NIE bestätigen."""
    s = _leer()
    with pytest.raises(ValueError, match="fail-closed"):
        ST.append_event(s, feld_id="agb_aufwendungen", wert=120000, zustand="bestaetigt",
                        herkunft={"herkunft": herkunft, "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber=schreiber, signal={"signal_1": {}, "signal_2": "KI-faelscht"},
                        katalog=KATALOG, ts=TS)


# ---------- S5: Feld-Katalog — ein Vorschlags-Schreiber darf kein HUMAN-ONLY-Feld vorschlagen ----------
@pytest.mark.parametrize("feld", [
    # "veranlagung" (Wahlrecht §26) stand hier bis 2026-08-12 und ist auf Julius' Entscheid
    # für die KI FREIGEGEBEN worden — Begründung und Gegen-Gates in
    # tests/test_veranlagung_llm_freigabe.py. Die Wahlrechts-Klasse bleibt hier abgedeckt
    # (antrag_ermaessigter_satz); die Freigabe gilt ausdrücklich nur für dieses eine Feld.
    "antrag_ermaessigter_satz", # Wahlrecht (§34 Abs.3)
    "kein_kap",                 # ⭐ Abwesenheits-Erklärung (K2-scharf)
    "gewinn_betriebsart",       # Rechts-Klassifikation
    "kap_gewinn_aktien",        # §20-Topf/Allokation
    "geburtsjahr",              # Identität
])
def test_s5_katalog_human_only_abgelehnt(feld):
    """Ein llm:-Schreiber, der ein HUMAN-ONLY-Feld setzen will → ValueError (Katalog fail-closed). Deckt das
    Abwesenheits-Prinzip (kein_kap: eine halluzinierte Abwesenheit = Under-Deklaration) + Wahlrecht + Allokation
    + Identität."""
    s = _leer()
    with pytest.raises(ValueError, match="Katalog"):
        _vorschlag(s, feld, 1, "llm:chat", "llm_vorschlag")


def test_s5b_katalog_suggestible_erlaubt():
    """Gegenprobe: llm→agb_aufwendungen, berechnet:maps→ep_entfernung_km, import:kontoauszug→spenden_betrag,
    import:beleg→bruttoarbeitslohn gehen durch (vorläufig). Kein Über-Blocken der erlaubten Vorschläge."""
    assert _vorschlag(_leer(), "agb_aufwendungen", 120000, "llm:chat", "llm_vorschlag")["zustand"] == "vorlaeufig"
    assert _vorschlag(_leer(), "ep_entfernung_km", 30, "berechnet:maps", "berechnet")["zustand"] == "vorlaeufig"
    assert _vorschlag(_leer(), "spenden_betrag", 50000, "import:kontoauszug", "kontoauszug")["zustand"] == "vorlaeufig"
    assert _vorschlag(_leer(), "bruttoarbeitslohn", 4500000, "import:beleg", "beleg_import")["zustand"] == "vorlaeufig"


def test_s5c_katalog_fehlt_fail_closed():
    """Ein Vorschlags-Schreiber OHNE katalog → ValueError (mandatory für ^llm:/^import:beleg/^import:kontoauszug/
    ^berechnet:maps). Kein Bypass durch Weglassen des Katalogs."""
    s = _leer()
    with pytest.raises(ValueError, match="Katalog"):
        ST.append_event(s, feld_id="agb_aufwendungen", wert=1, zustand="vorlaeufig",
                        herkunft={"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber="llm:chat", signal={"signal_1": None, "signal_2": None}, ts=TS)  # kein katalog=


# ---------- S6: die Steuer-Summen-Eingabe ändert sich ERST mit dem Confirm (Mapping-e2e) ----------
def test_s6_steuersumme_eingabe_invariant():
    """Der Kern-Invariant an der Steuer-Gate-Ebene: ein llm-Vorschlag spenden_betrag ist NICHT in der
    deklaration (die der Ring als Eingabe liest) — die Bemessung ist unverändert. Erst nach Confirm erscheint
    der Wert. (Die volle /ergebnis-est-e2e unter der echten Ring-Rechnung liegt in test_haut_chat.py, dev-1.)"""
    s = _leer()
    ev = _vorschlag(s, "spenden_betrag", 500000, "import:kontoauszug", "kontoauszug")
    felder, _ = ST.materialisiere(s)
    erg_vor = EM.deklariere(felder, BINDUNG)
    assert "spenden_betrag" in {u["feld_id"] for u in erg_vor["unvollstaendig"]}  # vorläufig → NICHT Steuer-Eingabe
    assert erg_vor["vollstaendig"] is False
    ST.append_event(s, feld_id="spenden_betrag", wert=500000, zustand="bestaetigt",
                    herkunft={"herkunft": "nutzer", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                    schreiber="ui:laie", signal={"signal_1": None, "signal_2": "confirm@spende"},
                    ersetzt=ev["event_id"], ts=TS)
    felder2, _ = ST.materialisiere(s)
    erg2 = EM.deklariere(felder2, BINDUNG)
    assert "spenden_betrag" not in {u["feld_id"] for u in erg2["unvollstaendig"]}  # jetzt gültige Eingabe


# ---------- S7: Provenance-Guard SCHREIBER-scoped (nicht herkunft-scoped) ----------
def test_s7_provenance_guard_schreiber_scoped():
    """[[produkt-beleg-writer]]-Lehre: der beleg_import-vorläufig-Zwang hängt an ^import:beleg, NICHT an der
    herkunft. Ein import:elster-Schreiber (echter ELSTER-Kanal, kein Vorschlag) mit herkunft=beleg_import DARF
    bestätigen — er ist NICHT vom import:beleg-Guard (noch vom Katalog: import:elster → _vorschlag_typ=None) erfasst."""
    s = _leer()
    ev = ST.append_event(s, feld_id="vor_an_anteil_rv", wert=3500000, zustand="bestaetigt",
                         herkunft={"herkunft": "beleg_import", "pruef_tiefe": "plausibilisiert", "haftung": "nutzer"},
                         schreiber="import:elster", signal={"signal_1": None, "signal_2": "lstb_z23"}, ts=TS)
    assert ev["zustand"] == "bestaetigt"   # import:elster ist ein echter Kanal, kein katalog-restringierter Vorschlag


# ---------- S8 (Instructor-Auflage): vorläufiger Abwesenheits-Flag unterdrückt die Frage NICHT ----------
def test_s8_vorjahr_abwesenheitsflag_vorlaeufig_unterdrueckt_nicht():
    """⭐ K2-SCHARF (import:vorjahr-Ausnahme): der Traverser excludiert eine Regel NUR bei einem BESTÄTIGTEN
    Gate mit wert=False (relevanz Z.88-95: `not _unbeantwortet(ev) and ev.wert is False`); ein VORLÄUFIGES Gate
    ist `_unbeantwortet`=True → „offen", NIE ausgeschlossen. Also: vorjahr überträgt den Abwesenheits-Flag
    kein_kap VORLÄUFIG → das Gate bleibt offen → die Folge-Regel/Frage wird NICHT unterdrückt → keine stille
    Under-Deklaration. Erst der bestätigte Flag (Confirm = 2. Signal) lässt das Gate greifen.

    Getestet an kein_kap (geltungsbedingung keine_kapitaleinkuenfte, regel p2_einkunftsarten): der wert=False-Zweig
    ist der, der excludiert — vorläufig-False darf NICHT excludieren, bestätigt-False schon."""
    rid = BINDUNG["kein_kap"]["quelle"]["regel_id"]   # p2_einkunftsarten
    # (a) VORLÄUFIGES Gate (egal welcher Wert) → NICHT ausgeschlossen (offen)
    s = _leer()
    ST.append_event(s, feld_id="kein_kap", wert=False, zustand="vorlaeufig",
                    herkunft={"herkunft": "vorjahr", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                    schreiber="import:vorjahr", signal={"signal_1": {"typ": "vorjahr"}, "signal_2": None}, ts=TS)
    assert TR.relevanz(s, BINDUNG)[rid]["status"] != "ausgeschlossen", \
        "vorläufiges Gate darf die Folge-Frage NICHT unterdrücken"
    # (b) BESTÄTIGTES Gate mit wert=False → JETZT ausgeschlossen (das 2. Signal lässt das Gate greifen)
    s2 = _leer()
    ST.append_event(s2, feld_id="kein_kap", wert=False, zustand="bestaetigt",
                    herkunft={"herkunft": "nutzer", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                    schreiber="ui:laie", signal={"signal_1": None, "signal_2": "confirm@kein_kap"}, ts=TS)
    assert TR.relevanz(s2, BINDUNG)[rid]["status"] == "ausgeschlossen"


# ---------- S9 (Ring-Instanz-Invariant, dev-1+dev-2-Fund 2026-07-20): vorläufige Instanz leckt NICHT ----------
def test_s9_instanz_vorlaeufig_kein_ring_leck():
    """⭐ RING-INSTANZ-INVARIANT: ein VORLÄUFIGES Instanz-Feld (gwg via EM.instanzen, das den STORE SEPARAT liest
    — der flat _bescheid_fn-bestätigt-Filter greift dort NICHT) darf die festgesetzte Steuer NICHT bewegen. Der
    Σ-Konsument (_gwg_sofortabzug_summe) MUSS vorläufige Instanzen ausschließen (per-Instanz zustand==bestaetigt,
    EM.instanzen liefert den meet). Ergänzt dev-1s flat-Golden (test_haut_chat) auf dem INSTANZ-Pfad. Erst Confirm
    zählt. (Empirisch war der Leak 600€ VOR dem Fix — dieser Golden fängt die Regression.)"""
    import api  # noqa: E402
    s = _leer()
    ev = ST.append_event(s, feld_id="gwg_anschaffungskosten_netto", wert=60000, zustand="vorlaeufig",
                         herkunft={"herkunft": "beleg_import", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                         schreiber="import:beleg", signal={"signal_1": {"typ": "beleg"}, "signal_2": None},
                         katalog=KATALOG, ts=TS)
    felder, _ = ST.materialisiere(s)
    felder_best = {fid: e for fid, e in felder.items() if e.get("zustand") == "bestaetigt"}
    assert api._gwg_sofortabzug_summe(felder_best, s, BINDUNG) == 0, \
        "vorläufige gwg-Instanz darf die festgesetzte Steuer NICHT bewegen (Ring-Instanz-Leck)"
    # Confirm (ersetzt das vorläufige Event) → JETZT zählt der Sofortabzug
    ST.append_event(s, feld_id="gwg_anschaffungskosten_netto", wert=60000, zustand="bestaetigt",
                    herkunft={"herkunft": "nutzer", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                    schreiber="ui:laie", signal={"signal_1": None, "signal_2": "confirm@gwg"},
                    ersetzt=ev["event_id"], ts=TS)
    felder2, _ = ST.materialisiere(s)
    felder2_best = {fid: e for fid, e in felder2.items() if e.get("zustand") == "bestaetigt"}
    assert api._gwg_sofortabzug_summe(felder2_best, s, BINDUNG) == 600   # 60000ct netto ≤800€ → Sofortabzug 600€


# ---------- S10 (Dual-Invariant UX-Seite, gwg-Instanz): /stand-Estimate-Pfad ZEIGT die vorläufig-Wirkung ----------
def test_s10_instanz_stand_estimate_zeigt_vorlaeufig():
    """⭐ DUAL-INVARIANT (Instructor 2026-07-20, UX-Gegenstück zu S9): der Estimate-Pfad (/stand-Range,
    nur_bestaetigt=False) INKLUDIERT die vorläufige gwg-Instanz (Range zeigt „bestätige gwg → −600€"), WÄHREND
    der Festgesetzt-Pfad (/ergebnis, default nur_bestaetigt=True, S9) sie IGNORIERT. So bleibt die Live-Preview-UX
    erhalten, OHNE das Security-Leck wiederzuöffnen (/stand emittiert nie die festgesetzte Steuer)."""
    import api  # noqa: E402
    s = _leer()
    _vorschlag(s, "gwg_anschaffungskosten_netto", 60000, "import:beleg", "beleg_import")
    felder, _ = ST.materialisiere(s)
    felder_best = {fid: e for fid, e in felder.items() if e.get("zustand") == "bestaetigt"}
    # Festgesetzt (default True) → vorläufig raus → 0 (= S9-Security)
    assert api._gwg_sofortabzug_summe(felder_best, s, BINDUNG, nur_bestaetigt=True) == 0
    # Estimate (/stand, False) → vorläufig DRIN → 600 (Range zeigt das Potenzial)
    assert api._gwg_sofortabzug_summe(felder_best, s, BINDUNG, nur_bestaetigt=False) == 600


# ---------- S11 (Instructor-Fund): oepnv_kosten_jahr Cent→Euro-Naht, kein 100×-Under-tax ----------
def test_s11_oepnv_kosten_jahr_cent_zu_euro_kein_100x():
    """⭐ NAHT-BUG: ep_oepnv_kosten kommt aus dem Store in CENT (bindung typ=cent), der Runner-Accessor
    (golden/runner.catala_entfernungspauschale) erwartet EURO — ohne //100 an der Slot-Übergabe wird eine
    Cent-Eingabe 100× zu groß gelesen (1380€ Jobticket-Kosten kämen als 138000€ Werbungskosten an →
    Steuer→0). Golden ep_2024_beispiel1_oepnv (220 Tage, 20 km, kein Kfz, ÖPNV 1380€ > Pauschale 1320€ →
    ÖPNV-Günstiger greift) beweist die korrekte Größenordnung end-to-end über den echten api._bescheid_fn-
    Slot-Pfad. War VOR dem Fix maskiert, weil jeder bestehende Test oepnv_kosten_jahr=0 setzt (0 // 100 == 0)."""
    import api  # noqa: E402
    try:
        import runner  # noqa: F401
    except Exception as e:
        pytest.skip(f"Catala-Toolchain nicht verfügbar: {type(e).__name__}: {e}")
    fn = api._bescheid_fn("abziehbarer_betrag", 2024, BINDUNG)
    assert fn is not None
    zahl = fn({"ep_arbeitstage": 220, "ep_entfernung_km": 20,
              "ep_oepnv_kosten": 138000, "ep_eigenes_kfz": False})
    assert zahl == 138000, f"1380€ ÖPNV-Kosten müssen 138000 Cent ergeben, nicht {zahl}"
    assert zahl != 13800000, "100×-Regression: Cent-Rohwert ungeteilt als Euro gelesen"


# ---------- S12 (Lead-Fund): § 6 Abs. 2 GWG-800€-Schwelle, Cent-Trunkierung an der Grenze ----------
def test_s12_gwg_schwelle_800eur_cent_trunkierung():
    """⭐ NAHT-BUG: _abzug rechnete netto//100 (Euro-Floor) VOR dem catala-≤800-Vergleich → 800,01-800,99€
    (80001-80099 Cent) rundete auf 800 ab und ging fälschlich als GWG durch (Sofortabzug statt AfA,
    under-tax). Fix: Cent-Schwellen-Guard VOR der Euro-Rundung. Grenzfälle: 80000ct (=800,00€, voller
    Sofortabzug), 80001ct (=800,01€, GENAU über der Schwelle → kein GWG), 79999ct (knapp drunter → GWG)."""
    import api  # noqa: E402
    try:
        import runner  # noqa: F401
    except Exception as e:
        pytest.skip(f"Catala-Toolchain nicht verfügbar: {type(e).__name__}: {e}")
    assert api._gwg_sofortabzug_summe({"gwg_anschaffungskosten_netto": {"wert": 80000}}, None, None) == 800, \
        "800,00€ (Schwelle exakt) muss voller Sofortabzug bleiben"
    assert api._gwg_sofortabzug_summe({"gwg_anschaffungskosten_netto": {"wert": 80001}}, None, None) == 0, \
        "800,01€ (1 Cent über der Schwelle) darf NICHT mehr als GWG durchgehen (war Under-tax-Bug)"
    assert api._gwg_sofortabzug_summe({"gwg_anschaffungskosten_netto": {"wert": 79999}}, None, None) == 799, \
        "799,99€ (knapp unter der Schwelle) bleibt GWG (Sofortabzug)"
