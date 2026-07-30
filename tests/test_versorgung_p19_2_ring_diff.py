"""§ 19 Abs. 2 Versorgungsfreibetrag + Zuschlag — Ring-Differential-Tests.

Ring-Case: Privatfall mit/ohne Versorgungsbezug (Pension Beamter, Betriebsrente) →
Steuerlast sinkt mit Freibetrag. Verifiziert, dass Ring-Verdrahtung funktioniert und
VFB wirklich angerechnet wird (nicht stille 0).

STATUS: SKELETT mit Hard-Coded-Inputs (Ring-Binding TBD, s.u.)
"""
import os
import sys
import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "golden"))


def _runner():
    """Fallback für Catala-Toolchain nicht verfügbar."""
    try:
        import runner
        return runner
    except Exception as e:
        pytest.skip(f"Catala-Toolchain nicht verfügbar: {type(e).__name__}: {e}")


# ========== RING-DIFFERENTIAL-TESTS ==========

@pytest.mark.skip(reason="Ring-Binding für versorgungsbezuege-Felder TBD (Julius-Klärung ausstehend)")
def test_ring_differential_versorgungsbezug_vs_ohne():
    """Ring-Differential: Privatfall OHNE → MIT Versorgungsbezug (Pension) →
    Steuerlast sinkt (Freibetrag angerechnet).

    SZENARIO: Rentner, 2025, nur Pension (keine weitere Einkünfte).
    - Ohne VFB: zvE = Pension, EST(zvE).
    - Mit VFB: zvE' = Pension - VFB - Zuschlag, EST(zvE') < EST(zvE).

    INPUTS (Hard-Coded, warten auf Ring-Binding):
    - veranlagungszeitraum: 2025
    - veranlagung: einzel
    - geburtsjahr: 1960 (Alters-Gate: ≥ 63 Lj erfüllt)
    - versorgungsbezuege_jahresrente: 24000 EUR (2000 € Monatsbasis)
    - versorgungsbeginn_jahr: 2020 (Kohorte: 16.0% / HB 1200 / Zuschlag 360)
    - versorgungsbezuege_sind_beamtenrechtlich: true (Beamten-Ruhegehalt, kein Alters-Gate)
    - versorgungsbezuege_bemessungsgrundlage: 24000 EUR (12 × 2000)
      [OFFEN: wird diese vom Ring aus Monatsbetrag berechnet, oder Input?]

    ERGEBNIS-CHECKS:
    1. catala_p19_2_versorgungsfreibetrag({...}) → VFB ≈ 1200 + 360 = 1560 EUR
    2. gesamt_ring(OHNE VB) → est_1
    3. gesamt_ring(MIT VB, VFB=1560) → est_2
    4. est_2 < est_1 (Steuersenkung durch Freibetrag)
    5. (est_1 - est_2) ≈ EST-Satz × VFB (grob, nicht exakt wegen Progression)

    ABHÄNGIGKEIT: Ring-Accessor muss catala_p19_2_versorgungsfreibetrag in gesamt integrieren
    (wie catala_renten_einkuenfte für § 22), versorgungsbezuege_* als Inputs vom Frontend
    erhalten + in api.py verdrahten (Gate VersorgungsfreibetragOffen).
    """
    runner = _runner()

    # Case 1: OHNE Versorgungsbezug
    fall_ohne_vb = {
        "veranlagungszeitraum": 2025,
        "veranlagung": "einzel",
        "arbeitslohn": 0,
        # Versorgungsbezüge-Felder NICHT gesetzt
        # → gesamt-Ring behandelt sie als absent → 0 Beitrag
    }
    # est_ohne_vb = runner.catala_gesamt(fall_ohne_vb)["festzusetzende_est"]

    # Case 2: MIT Versorgungsbezug (Pension)
    fall_mit_vb = {
        "veranlagungszeitraum": 2025,
        "veranlagung": "einzel",
        "geburtsjahr": 1960,
        "arbeitslohn": 0,
        # Neue Felder (Ring-Binding):
        "versorgungsbezuege_jahresrente": 2400000,  # cent: 24000 EUR
        "versorgungsbeginn_jahr": 2020,
        "versorgungsbezuege_sind_beamtenrechtlich": True,
        "versorgungsbezuege_bemessungsgrundlage": 2400000,  # 12 × 200000 cent/monat
    }
    # est_mit_vb = runner.catala_gesamt(fall_mit_vb)["festzusetzende_est"]

    # ASSERTIONS (alle warten auf Ring-Integration):
    # assert est_mit_vb < est_ohne_vb, "VFB sollte Steuer senken"
    # steuersenkung = est_ohne_vb - est_mit_vb
    # vfb_expected = 1560  # 2020-Kohorte: 16% × 24000 = 3840 gedeckelt auf 1200 + Zuschlag 360
    # est_satz = 42  # grob, Spitzensatz 2025
    # assert steuersenkung > vfb_expected * est_satz // 100 * 0.5  # konservativ: mind. 50% ESt-Satz


@pytest.mark.skip(reason="Ring-Binding TBD")
def test_ring_differential_altersgate_nicht_beamtenrechtlich():
    """Ring-Differential: Nicht-beamtenrechtliche Versorgung (z.B. Betriebsrente private GmbH).
    Alters-Gate 63 Lj: Unter 63 → KEIN VFB (Gate sperrt in api.py).

    SZENARIO: Privatangestellte, Betriebsrente mit 55 Jahren begonnen → keine Berechtigung (< 63).
    - versorgungsbezuege_sind_beamtenrechtlich: false
    - geburtsjahr: 1970 (55 Jahre im VZ 2025 → 60. Geburtstag 2030 → 2025 noch < 60/63)
    - → VersorgungsfreibetragOffen (Gate stoppt, kein stilles 0)

    ABHÄNGIGKEIT: api.py muss altersgate-Gate implementieren (fail-closed).
    """
    pass


@pytest.mark.skip(reason="Ring-Binding TBD")
def test_ring_differential_altersgate_schwerbehindert():
    """Ring-Differential: Nicht-beamtenrechtliche Versorgung MIT Schwerbehinderung.
    Alters-Gate 60 Lj (nicht 63) bei Schwerbehinderung (grad_der_behinderung ≥ 50).

    SZENARIO: Private Betriebsrente, mit 58 Jahren begonnen, schwerbehindert.
    - versorgungsbezuege_sind_beamtenrechtlich: false
    - geburtsjahr: 1967 (58 Jahre im VZ 2025 → 60. Geburtstag 2027 → erfüllt)
    - grad_der_behinderung: 60 (Schwerbehinderung)
    - → Gate passt, VFB wird gewährt

    ABHÄNGIGKEIT: Alters-Gate muss grad_der_behinderung prüfen (fail-closed bei Fehlen).
    """
    pass


@pytest.mark.skip(reason="Ring-Binding TBD")
def test_ring_differential_freibetrag_fixierung_folgejahr():
    """Ring-Differential: Freibetrag-Fixierung wie § 22 Rentenfreibetrag.

    SZENARIO: Erste Versorgung 2024, VZ 2024 + 2025.
    - VZ 2024 (Erstjahr): VFB = % × BG, Zuschlag (berechnet aus Kohorte).
    - VZ 2025 (Folgejahr): VFB bleibt FIXIERT (2024-Betrag), nicht neu berechnet
      (auch wenn Pension 2025 erhöht wurde). → § 19 Abs. 2 S. 8-9

    SZENARIO:
    - VZ 2024: Pension 24000, VFB = 1020 + 306 = 1326 → bescheid
    - VZ 2025: Pension 24600 (erhöht um 2.5%), aber VFB = 1326 (fixiert, nicht 1350)
    - zvE 2025 = 24600 - 1326 = 23274 (nicht 24600 - 1350 = 23250)
      → Rentenerhöhung ist voll steuerpflichtig (§ 19 Abs. 2 S. 8-9)

    RING-ANFORDERUNG: versorgungsbezuege_freibetrag-Feld (analog rentenfreibetrag),
    NOT-askable im Erstjahr (Ring rechnet), askable + vorjahr=vorschlag ab Folgejahr
    (Nutzer gibt Wert aus Vorjahrs-Bescheid ein). Gate:
    falls versorgungsbeginn_jahr < VZ AND versorgungsbezuege_freibetrag absent
    → VersorgungsfreibetragFixierungOffen (fail-closed).

    ABHÄNGIGKEIT: Feld versorgungsbezuege_freibetrag in bindung (wie rentenfreibetrag).
    """
    pass


# ========== MUTATION-TEST ==========

@pytest.mark.skip(reason="Ring-Binding TBD")
def test_mutation_versorgungsfreibetrag_ring_verdrahtung():
    """Mutation-Probe: Ring-Verdrahtung des VFB rausnehmen (Zeile auf 0 setzen),
    Differential-Test läuft → MUSS ROT werden (sonst ist die Verdrahtung tot).

    VORGEHEN:
    1. produkt/haut/api.py: Zeile, die catala_p19_2_versorgungsfreibetrag aufruft
       + in zvE einspeist, identifizieren.
    2. Zeile auf 0 setzen (Kurztest: VFB wird ignoriert).
    3. test_ring_differential_versorgungsbezug_vs_ohne() laufen.
    4. ERWARTUNG: est_mit_vb == est_ohne_vb (VFB hat keine Wirkung) → TEST ROT.
    5. Mutation zurückbauen, Test grün.

    ANMERKUNG: Diese Probe läuft als Finalisierungs-Step NACH Ring-Integration.
    """
    pass
