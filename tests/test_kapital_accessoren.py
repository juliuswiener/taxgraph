"""Layer-II Kapital — Accessor-Gate (§20/§32d full-pipeline). Deterministisch, NULL LLM.

Validiert dev-1s Kapital-Accessoren (golden/runner.py) gegen die hand-transkribierten Registry-Formeln
(p20_9 Sparer-PB, p20_6 Verlustverrechnung, p32d_1 Abgeltung/Günstiger) UND die Full-Pipeline-Komposition
(Sparer-PB → 2× catala_gesamt → Kapital-Steuer → § 2-Addition), Zielwerte aus dem Instructor-fixierten
Kapital-Zuschnitt (K4=16174 brutto-basiert, reiner-Kapital=0 via Günstiger). EURO.

Der golden-runner hat KEINEN deklarativen kap-Feld-Dispatch (catala_est→gesamtfall→catala_gesamt liest
steuer_kapital_gesondert vorberechnet); die Full-Pipeline (kap_*→steuer) läuft über dev-1s gesamt-Ring
(HTTP-e2e test_kombiniert_job_und_kapital). Dieser Gate spiegelt sie deterministisch auf Accessor-Ebene.
"""
from __future__ import annotations

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "golden"))
import runner as R   # noqa: E402

VZ = 2025


# ---- p20_9 Sparer-Pauschbetrag (max(0, kapitalertraege − 1000/2000)) ----------

def test_sparer_pb_einzel():
    assert R.catala_sparer_pb({"veranlagungszeitraum": VZ, "kapitalertraege": 10000}) == 9000


def test_sparer_pb_zusammen_verdoppelt():
    assert R.catala_sparer_pb({"veranlagungszeitraum": VZ, "kapitalertraege": 12000,
                               "zusammenveranlagung": True}) == 10000       # § 20 Abs. 9 S. 3: 2000


def test_sparer_pb_absorbiert_unter_freibetrag():
    assert R.catala_sparer_pb({"veranlagungszeitraum": VZ, "kapitalertraege": 500}) == 0


# ---- p20_6 Verlustverrechnung (getrennte Töpfe, je Boden 0) ------------------

def test_verrechnung_topf_trennung():
    # sonstige-Verlust 3000 mindert NICHT den Aktien-Gewinn: saldo_aktien=3000, saldo_sonstige=0
    assert R.catala_kapital_verrechnung({"gewinn_aktien": 5000, "verlust_aktien": 2000,
                                         "gewinn_sonstige": 1000, "verlust_sonstige": 3000}) == 3000


def test_verrechnung_beide_toepfe_positiv():
    # saldo_aktien=max(0,5000−1000)=4000, saldo_sonstige=max(0,2000−500)=1500 → 5500
    assert R.catala_kapital_verrechnung({"gewinn_aktien": 5000, "verlust_aktien": 1000,
                                         "gewinn_sonstige": 2000, "verlust_sonstige": 500}) == 5500


# ---- p32d_1 Abgeltung / Günstiger (min(0.25×kap, est_mit−est_ohne)) ----------

def test_steuer_abgeltung_gewinnt():
    # abgeltung=2250 < delta=3651 → Abgeltung (25 %) greift
    assert R.catala_kapital_steuer({"veranlagungszeitraum": VZ, "kapitaleinkuenfte": 9000,
                                    "est_regulaer_mit_kap": 18052, "est_regulaer_ohne_kap": 14401}) == 2250


def test_steuer_guenstiger_gewinnt():
    # delta=902 < abgeltung=1000 → Günstiger (tariflich) greift
    assert R.catala_kapital_steuer({"veranlagungszeitraum": VZ, "kapitaleinkuenfte": 4000,
                                    "est_regulaer_mit_kap": 902, "est_regulaer_ohne_kap": 0}) == 902


# ---- Full-Pipeline-Komposition (Sparer-PB → 2× catala_gesamt → Steuer → §2) ---

def _kapital_gesamt(einkuenfte_ns: int, kapitalertraege: int, veranlagung: str = "einzel") -> int:
    """Die exakte Accessor-Kette wie dev-1s Ring: Sparer-PB, dann est mit/ohne Kapital tariflich, dann
    min(Abgeltung, delta), zur ESt-ohne-Kapital addiert (§ 2 + § 32d Abs. 3 S. 2). EURO."""
    kap_netto = R.catala_sparer_pb({"veranlagungszeitraum": VZ, "kapitalertraege": kapitalertraege,
                                    "zusammenveranlagung": veranlagung == "zusammen"})
    base = {"gesamtfall": True, "veranlagungszeitraum": VZ, "veranlagung": veranlagung,
            "einkuenfte_nichtselbststaendig": einkuenfte_ns}
    est_ohne = R.catala_gesamt(base)
    est_mit = R.catala_gesamt({**base, "einkuenfte_kapitalvermoegen": kap_netto})
    kapital_steuer = R.catala_kapital_steuer({"veranlagungszeitraum": VZ, "kapitaleinkuenfte": kap_netto,
                                              "est_regulaer_mit_kap": est_mit, "est_regulaer_ohne_kap": est_ohne})
    return est_ohne + kapital_steuer


def test_full_pipeline_k4_abgeltung_16174():
    # K4 brutto 60000 → § 9a → einkünfte 58770; kap 10000 → Sparer-PB → 9000; Abgeltung 2250 gewinnt.
    assert _kapital_gesamt(einkuenfte_ns=58770, kapitalertraege=10000) == 16174


def test_full_pipeline_reiner_kapital_guenstiger_null():
    # reiner Kapital (kein Job, kap 5000): Grundtarif auf Klein-Kapital = 0 → Günstiger → 0
    assert _kapital_gesamt(einkuenfte_ns=0, kapitalertraege=5000) == 0
