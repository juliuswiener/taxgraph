"""Unit-Tests für §101 Mobilitätsprämie-Accessor (catala_p101_mobilitaetspraemie)."""


# Inline accessor (kein Catala-Import nötig für reine Python-Tests) — 1:1 zu runner.catala_p101_mobilitaetspraemie
def catala_p101_mobilitaetspraemie(s: dict) -> int:
    """§ 101 EStG — Mobilitätsprämie: 14 % der Bemessungsgrundlage (S. 4). EURO int.
    Basis = EP ab 21 km; S. 3 AN-Pauschbetrag-soweit (nur bei ist_arbeitnehmer); S. 2 GFB-Unterschreitung."""
    ep_ab_21 = int(s.get("entfernungspauschale_ab_21km", 0))
    zvE = int(s.get("zu_versteuerndes_einkommen", 0))
    gfb = int(s.get("grundfreibetrag", 0))
    if s.get("ist_arbeitnehmer", False):                              # § 101 S. 3
        wk_gesamt = int(s.get("werbungskosten_gesamt", 0))
        an_pausch = int(s.get("arbeitnehmer_pauschbetrag", 0))
        ep_ab_21 = min(ep_ab_21, max(0, wk_gesamt - an_pausch))
    unterschreitung = max(0, gfb - zvE)                              # § 101 S. 2
    bemessungsgrundlage = min(ep_ab_21, unterschreitung)
    return bemessungsgrundlage * 14 // 100                           # § 101 S. 4


def test_p101_grundfall_unter_grundfreibetrag():
    """Geringverdiener mit EP ab 21km unter Grundfreibetrag.
    zvE=8000, GFB=11600, EP_ab_21=3000 → Prämie = min(3000, 3600)*14% = 420€."""
    result = catala_p101_mobilitaetspraemie({
        "entfernungspauschale_ab_21km": 3000,
        "zu_versteuerndes_einkommen": 8000,
        "grundfreibetrag": 11600,
    })
    assert result == 420, f"Expected 420, got {result}"


def test_p101_keine_ep():
    """Keine Entfernungspauschale ab 21km → Prämie = 0."""
    result = catala_p101_mobilitaetspraemie({
        "entfernungspauschale_ab_21km": 0,
        "zu_versteuerndes_einkommen": 8000,
        "grundfreibetrag": 11600,
    })
    assert result == 0, f"Expected 0, got {result}"


def test_p101_zvE_ueber_grundfreibetrag():
    """zvE übersteigt Grundfreibetrag → keine Prämie (nicht Geringverdiener)."""
    result = catala_p101_mobilitaetspraemie({
        "entfernungspauschale_ab_21km": 3000,
        "zu_versteuerndes_einkommen": 12000,
        "grundfreibetrag": 11600,
    })
    assert result == 0, f"Expected 0, got {result}"


def test_p101_ep_kleiner_unterschreitung():
    """EP ab 21km kleiner als Unterschreitung.
    zvE=5000, GFB=11600, EP_ab_21=2000 → Prämie = min(2000, 6600)*14% = 280€."""
    result = catala_p101_mobilitaetspraemie({
        "entfernungspauschale_ab_21km": 2000,
        "zu_versteuerndes_einkommen": 5000,
        "grundfreibetrag": 11600,
    })
    assert result == 280, f"Expected 280, got {result}"


def test_p101_ep_groesser_unterschreitung():
    """EP ab 21km größer als Unterschreitung.
    zvE=10000, GFB=11600, EP_ab_21=5000 → Prämie = min(5000, 1600)*14% = 224€."""
    result = catala_p101_mobilitaetspraemie({
        "entfernungspauschale_ab_21km": 5000,
        "zu_versteuerndes_einkommen": 10000,
        "grundfreibetrag": 11600,
    })
    assert result == 224, f"Expected 224, got {result}"


def test_p101_s3_an_pauschbetrag_soweit():
    """§ 101 S. 3: bei AN zählt EP ab 21km nur SOWEIT WK gesamt den AN-Pauschbetrag (1230) übersteigt.
    ep_ab21=1000, WK gesamt=1500, Pauschbetrag=1230 → soweit=270; zvE=10000, GFB=12000 → Unterschr.=2000.
    Bemessung = min(1000, 270) then min(270, 2000) = 270 → 270*14//100 = 37€."""
    result = catala_p101_mobilitaetspraemie({
        "entfernungspauschale_ab_21km": 1000,
        "ist_arbeitnehmer": True,
        "werbungskosten_gesamt": 1500,
        "arbeitnehmer_pauschbetrag": 1230,
        "zu_versteuerndes_einkommen": 10000,
        "grundfreibetrag": 12000,
    })
    assert result == 37, f"Expected 37, got {result}"


def test_p101_s3_wk_unter_pauschbetrag_keine_praemie():
    """§ 101 S. 3: WK gesamt ≤ AN-Pauschbetrag → EP vom Pauschbetrag vollständig absorbiert → keine Prämie.
    WK gesamt=1200 ≤ 1230 → soweit=0 → Bemessung 0."""
    result = catala_p101_mobilitaetspraemie({
        "entfernungspauschale_ab_21km": 1000,
        "ist_arbeitnehmer": True,
        "werbungskosten_gesamt": 1200,
        "arbeitnehmer_pauschbetrag": 1230,
        "zu_versteuerndes_einkommen": 10000,
        "grundfreibetrag": 12000,
    })
    assert result == 0, f"Expected 0, got {result}"


def test_p101_nicht_an_keine_s3_kappung():
    """Nicht-AN (Betriebsausgaben, ist_arbeitnehmer absent/False): S. 3 entfällt, kein Pauschbetrag-Abzug.
    ep_ab21=3000, zvE=8000, GFB=11600 → min(3000, 3600)=3000 → 420€ (WK-Felder ignoriert)."""
    result = catala_p101_mobilitaetspraemie({
        "entfernungspauschale_ab_21km": 3000,
        "werbungskosten_gesamt": 1500,
        "arbeitnehmer_pauschbetrag": 1230,
        "zu_versteuerndes_einkommen": 8000,
        "grundfreibetrag": 11600,
    })
    assert result == 420, f"Expected 420, got {result}"


def test_p101_snapshot_seeds():
    """Snapshot-Seed (raster point).
    ep=4500, zvE=7500, GFB=11600 → min(4500, 4100)*0.14 = 4100*0.14 = 574€."""
    result = catala_p101_mobilitaetspraemie({
        "entfernungspauschale_ab_21km": 4500,
        "zu_versteuerndes_einkommen": 7500,
        "grundfreibetrag": 11600,
    })
    assert result == 574, f"Expected 574, got {result}"
