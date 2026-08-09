"""Klasse-3 fail-open — {k: slots[k] for k in (...) if k in slots} in api.py (an_gesamt/gesamt).

Auftrag Team-Lead (2026-08-09), Vorlage reports/adjudikation/slot_fn_fail_open_sweep_2026-08-08.md
Abschnitt "Klasse 3": zwei DictComp-Lesestellen in _bescheid_fn (quantitaet festzusetzende_est
und festzusetzende_est_gesamt) sahen fail-closed aus (Subscript-Zugriff), waren es aber nicht —
"if k in slots" liess einen fehlenden Key (arbeitstage/entfernung_km_roh/oepnv_kosten_jahr/
eigenes_oder_ueberlassenes_kfz) lautlos aus wk_input verschwinden statt zu werfen. Alle 4 Keys
stehen im Pflicht-Kegel BEIDER Scheiben (EP_FELDER, api_constants.py:19/SCHEIBEN) — der Filter
hatte keinen legitimen Anwendungsfall, ist inzwischen entfernt (api.py, 2026-08-09).

MESSUNG vor dem Fix (reports/adjudikation/klasse3_fail_open_2026-08-09.md):
- festzusetzende_est (an_gesamt): fehlender Key -> 323,00 EUR stille Steuermehrbelastung,
  KEIN Fehler (992200 ct -> 1024500 ct fuer denselben Fall).
- festzusetzende_est_gesamt (gesamt): fehlender Key -> 351,00 EUR stille Steuermehrbelastung,
  KEIN Fehler (1310100 ct -> 1345200 ct fuer denselben Fall). Deckt sich mit dem bereits
  bestehenden e2e-Paar test_kombiniert_mit_pendel_wk (1310100) / test_kombiniert_job_und_vermietung
  (1345200) in tests/test_paket_b_e2e_http.py.

Die zweite Ebene (golden/runner.py::catala_werbungskosten_n, eigenes "if X in s:" je WK-Komponente)
ist NICHT die fail-open-Ursache und bleibt unveraendert: sie traegt legitim optionale Komponenten
(dHf/Verpflegung/Uebernachtung/Arbeitsmittel werden in api.py bewusst bedingt in wk_input
geschrieben, siehe api.py ~Zeile 493ff). Die vier EP-Keys sind dort NICHT bedingt gedacht — mit dem
Fix an Ebene 1 stehen sie ab jetzt IMMER in wk_input (sonst KeyError vorher), Ebene 2 sieht sie
also nie mehr als fehlend fuer diese beiden Aufrufer. test_zweite_ebene_* unten dokumentiert das
Verhalten von Ebene 2 direkt, damit die Kopplung sichtbar bleibt.
"""
import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for sub in ("produkt/haut", "golden", "produkt/unsicherheit", "produkt/store",
            "produkt/traverser", "produkt/mapping"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API
import traverser as TR


def _catala_da() -> bool:
    try:
        import runner  # noqa: F401
        return True
    except Exception:
        return False


AN_GESAMT_FELDER = {
    "bruttoarbeitslohn": {"wert": 5000000, "zustand": "bestaetigt"},
    "veranlagung": {"wert": "einzel", "zustand": "bestaetigt"},
    "ep_arbeitstage": {"wert": 220, "zustand": "bestaetigt"},
    "ep_entfernung_km": {"wert": 30, "zustand": "bestaetigt"},
    "ep_oepnv_kosten": {"wert": 0, "zustand": "bestaetigt"},
    "ep_eigenes_kfz": {"wert": True, "zustand": "bestaetigt"},
    "vor_an_anteil_rv": {"wert": 0, "zustand": "bestaetigt"},
    "vor_ag_anteil_rv": {"wert": 0, "zustand": "bestaetigt"},
    "vor_rv_ausserhalb_lstb": {"wert": 0, "zustand": "bestaetigt"},
    "basis_kv": {"wert": 0, "zustand": "bestaetigt"},
    "basis_pv": {"wert": 0, "zustand": "bestaetigt"},
    "versicherungsart": {"wert": "gesetzlich_an", "zustand": "bestaetigt"},
    "mit_anspruch_auf_zuschuss": {"wert": False, "zustand": "bestaetigt"},
    "vorsorge_arbeitslosenversicherung": {"wert": 0, "zustand": "bestaetigt"},
    "vorsorge_erwerbsunfaehigkeit": {"wert": 0, "zustand": "bestaetigt"},
    "vorsorge_unfall_haftpflicht": {"wert": 0, "zustand": "bestaetigt"},
    "vorsorge_rv_alt_mit_ueberschuss": {"wert": 0, "zustand": "bestaetigt"},
    "vorsorge_rv_alt_ohne_ueberschuss": {"wert": 0, "zustand": "bestaetigt"},
    "fam_anzahl_kinder": {"wert": 0, "zustand": "bestaetigt"},
    "verlustvortrag_bestand": {"wert": 0, "zustand": "bestaetigt"},
    "dhf_im_inland": {"wert": True, "zustand": "bestaetigt"},
    "dhf_monate": {"wert": 0, "zustand": "bestaetigt"},
    "dhf_unterkunftskosten": {"wert": 0, "zustand": "bestaetigt"},
    "dhf_zweitmietvertrag": {"wert": False, "zustand": "bestaetigt"},
    "dhf_eigener_haushalt": {"wert": False, "zustand": "bestaetigt"},
    "dhf_keine_berufliche_veranlassung": {"wert": False, "zustand": "bestaetigt"},
    "vpf_tage_ganz": {"wert": 0, "zustand": "bestaetigt"},
    "vpf_tage_abwesend": {"wert": 0, "zustand": "bestaetigt"},
    "vpf_tage_abwesend_14h": {"wert": 0, "zustand": "bestaetigt"},
    "vpf_tage_abwesend_8h": {"wert": 0, "zustand": "bestaetigt"},
    "vpf_monate_am_ort": {"wert": 0, "zustand": "bestaetigt"},
}
AN_GESAMT_FELD_WERTE = {"bruttoarbeitslohn": 5000000, "veranlagung": "einzel",
                        "ep_arbeitstage": 220, "ep_entfernung_km": 30,
                        "ep_oepnv_kosten": 0, "ep_eigenes_kfz": True}

GESAMT_FELD_WERTE = {
    "vv_einnahmen": 1877000, "vv_gebaeude_afa": 0, "vv_schuldzinsen": 0,
    "vv_erhaltungsaufwand": 0, "vv_sonstige_wk": 0, "vv_entgelt_quote_prozent": 100,
    "veranlagung": "einzel", "bruttoarbeitslohn": 4000000,
    "vor_an_anteil_rv": 0, "vor_ag_anteil_rv": 0, "vor_rv_ausserhalb_lstb": 0,
    "versicherungsart": "gesetzlich_an",
    "basis_kv": 0, "basis_pv": 0, "vorsorge_arbeitslosenversicherung": 0,
    "vorsorge_erwerbsunfaehigkeit": 0, "vorsorge_unfall_haftpflicht": 0,
    "vorsorge_rv_alt_mit_ueberschuss": 0, "vorsorge_rv_alt_ohne_ueberschuss": 0,
    "mit_anspruch_auf_zuschuss": False,
    "ep_arbeitstage": 220, "ep_entfernung_km": 30, "ep_oepnv_kosten": 0, "ep_eigenes_kfz": True,
    "kap_kapitalertraege": 0, "kap_gewinn_aktien": 0, "kap_verlust_aktien": 0,
    "kap_gewinn_sonstige": 0, "kap_verlust_sonstige": 0,
    "kein_gewinn": True, "kein_kap": True, "kein_vuv": False, "kein_sonstige": True,
}


def test_an_gesamt_wirft_statt_still_zu_droppen_wenn_ep_entfernung_km_fehlt():
    """festzusetzende_est (api.py:483-487): fehlt ep_entfernung_km in feld_werte, MUSS bf()
    werfen (KeyError) statt die EP-Komponente lautlos zu unterschlagen (323 EUR gemessen)."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    bindung = TR.lade_bindung()
    bf = API._bescheid_fn("festzusetzende_est", 2025, bindung, AN_GESAMT_FELDER,
                           store=None, nur_bestaetigt=True)
    assert bf is not None
    mit = bf(AN_GESAMT_FELD_WERTE)
    assert mit == 992200, f"Regression an_gesamt mit EP: {mit} ct, erwartet 992200 ct"
    ohne = {k: v for k, v in AN_GESAMT_FELD_WERTE.items() if k != "ep_entfernung_km"}
    with pytest.raises(KeyError):
        bf(ohne)


def test_gesamt_wirft_statt_still_zu_droppen_wenn_ep_entfernung_km_fehlt():
    """festzusetzende_est_gesamt (api.py:742-745): dieselbe Pruefung fuer die gesamt-Scheibe
    (351 EUR gemessen). Regressionswerte decken sich mit test_paket_b_e2e_http.py
    (test_kombiniert_mit_pendel_wk == 1310100, test_kombiniert_job_und_vermietung == 1345200)."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    bindung = TR.lade_bindung()
    felder = {k: {"wert": v, "zustand": "bestaetigt"} for k, v in GESAMT_FELD_WERTE.items()}
    bf = API._bescheid_fn("festzusetzende_est_gesamt", 2025, bindung, felder,
                           store=None, nur_bestaetigt=True)
    assert bf is not None
    mit = bf(GESAMT_FELD_WERTE)
    assert mit == 1310100, f"Regression gesamt mit EP: {mit} ct, erwartet 1310100 ct"
    ohne = {k: v for k, v in GESAMT_FELD_WERTE.items() if k != "ep_entfernung_km"}
    with pytest.raises(KeyError):
        bf(ohne)


def test_zweite_ebene_catala_werbungskosten_n_bleibt_bewusst_permissiv():
    """golden/runner.catala_werbungskosten_n: "if X in s:" je Komponente ist der GENERELLE,
    legitime Mechanismus fuer optionale WK-Bestandteile (dHf/Verpflegung/Uebernachtung/AM sind in
    api.py bewusst bedingt in wk_input geschrieben). Dokumentiert nur das Verhalten — bleibt
    unveraendert, weil Ebene 1 (siehe oben) ab jetzt garantiert, dass die 4 EP-Keys nie mehr
    fehlen, wenn diese Funktion von den beiden Klasse-3-Stellen aus aufgerufen wird."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    import runner
    mit = {"veranlagungszeitraum": 2025, "arbeitstage": 220, "entfernung_km_roh": 30,
           "eigenes_oder_ueberlassenes_kfz": True, "oepnv_kosten_jahr": 0}
    ohne = {k: v for k, v in mit.items() if k != "entfernung_km_roh"}
    assert runner.catala_werbungskosten_n(mit) == 2156
    assert runner.catala_werbungskosten_n(ohne) == 0
