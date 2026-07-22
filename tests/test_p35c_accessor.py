"""Unit-Golden für die ECHTEN §35c-Accessoren (golden/runner.py), nicht Inline-Dummies.

§35c Abs.1 EStG: Sanierung 7 %/max 14.000 € (Abschluss- + nächstes Jahr), 6 %/max 12.000 €
(übernächstes Jahr); Energieberater (S.4) 50 %, aber gemäß BMF v. 2025-08-21 Rn. 56 „vom
(Gesamt-)Höchstbetrag und den Jahreshöchstbeträgen umfasst" (Jahresdeckel INKLUSIVE Energieberater).
Accessoren nehmen EUROS, geben EUROS. Deterministisch, NULL LLM.
"""
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "golden"))

import runner as R  # noqa: E402


class Test35cSanierung:
    """catala_p35c_sanierung: 7 %/6 %-Staffel mit Jahresdeckel 14.000/12.000 €."""

    def test_jahr1_unter_deckel(self):
        # Jahr 1 (ist_uebernaechstes=False): 20.000 € × 7 % = 1.400 € (unter Deckel)
        assert R.catala_p35c_sanierung({
            "sanierungsaufwendungen": 20000, "ist_uebernaechstes_foerderjahr": False}) == 1400

    def test_jahr1_am_deckel(self):
        # Jahr 1: 200.000 € × 7 % = 14.000 € = Jahreshöchstbetrag (exakt)
        assert R.catala_p35c_sanierung({
            "sanierungsaufwendungen": 200000, "ist_uebernaechstes_foerderjahr": False}) == 14000

    def test_jahr1_ueber_deckel_gekappt(self):
        # 215.000 € × 7 % = 15.050 € → gekappt auf 14.000 € (BMF Beispiel 10)
        assert R.catala_p35c_sanierung({
            "sanierungsaufwendungen": 215000, "ist_uebernaechstes_foerderjahr": False}) == 14000

    def test_jahr3_unter_deckel(self):
        # übernächstes Jahr: 20.000 € × 6 % = 1.200 € (unter Deckel)
        assert R.catala_p35c_sanierung({
            "sanierungsaufwendungen": 20000, "ist_uebernaechstes_foerderjahr": True}) == 1200

    def test_jahr3_am_deckel(self):
        # übernächstes Jahr: 200.000 € × 6 % = 12.000 € = Jahreshöchstbetrag
        assert R.catala_p35c_sanierung({
            "sanierungsaufwendungen": 200000, "ist_uebernaechstes_foerderjahr": True}) == 12000

    def test_null(self):
        assert R.catala_p35c_sanierung({
            "sanierungsaufwendungen": 0, "ist_uebernaechstes_foerderjahr": False}) == 0


class Test35cEnergieberater:
    """catala_p35c_energieberater: 50 % flat (§35c Abs.1 S.4, Abschlussjahr)."""

    def test_1000(self):
        assert R.catala_p35c_energieberater({"energieberater_aufwendungen": 1000}) == 500

    def test_3000(self):
        # BMF Beispiel 10: 3.000 € Energieberater → 1.500 € (50 %)
        assert R.catala_p35c_energieberater({"energieberater_aufwendungen": 3000}) == 1500

    def test_null(self):
        assert R.catala_p35c_energieberater({"energieberater_aufwendungen": 0}) == 0


class Test35cJahresdeckel:
    """catala_p35c_jahresdeckel: min(Sanierung + Energieberater, 14.000/12.000) — Energieberater
    IST im Jahresdeckel umfasst (BMF Rn. 56 „Jahreshöchstbetrag inklusive Energieberaterkosten")."""

    def test_bmf_beispiel10_kombi_am_deckel(self):
        # BMF Beispiel 10 VZ 2020: Sanierung 14.000 (gekappt) + Energieberater 1.500 → Deckel 14.000
        san = R.catala_p35c_sanierung({"sanierungsaufwendungen": 215000, "ist_uebernaechstes_foerderjahr": False})
        eb = R.catala_p35c_energieberater({"energieberater_aufwendungen": 3000})
        assert R.catala_p35c_jahresdeckel({
            "sanierung_ermaessigung": san, "energieberater_ermaessigung": eb,
            "ist_uebernaechstes_foerderjahr": False}) == 14000

    def test_kombi_unter_deckel_additiv(self):
        # Beide unter Deckel: 700 (Sanierung 10.000×7 %) + 1.000 (Energieberater 2.000×50 %) = 1.700
        assert R.catala_p35c_jahresdeckel({
            "sanierung_ermaessigung": 700, "energieberater_ermaessigung": 1000,
            "ist_uebernaechstes_foerderjahr": False}) == 1700

    def test_uebernaechstes_deckel_12000(self):
        # übernächstes Jahr: Deckel 12.000 auch für die Kombination
        assert R.catala_p35c_jahresdeckel({
            "sanierung_ermaessigung": 12000, "energieberater_ermaessigung": 1500,
            "ist_uebernaechstes_foerderjahr": True}) == 12000
