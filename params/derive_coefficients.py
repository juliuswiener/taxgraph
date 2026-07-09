"""Derive the closed-form § 32a Abs. 1 tariff coefficients per Veranlagungszeitraum.

The German income tax schedule (§ 32a Abs. 1 EStG) is published as a piecewise
polynomial in closed form, for example for VZ 2026:

    zone 2: (914.51 * y + 1400) * y
    zone 3: (173.10 * z + 2397) * z + 1034.87
    zone 4: 0.42 * x - 11135.63
    zone 5: 0.45 * x - 19470.38

These closed-form coefficients are fully determined by four numbers per zone
transition that the legislator fixes explicitly and that GETTSIM stores with a
BGBl reference in `einkommensteuertarif.yaml`:

  - the zone boundaries (Grundfreibetrag, upper bound of zone 2 and 3, start of
    the top proportional zone),
  - the entry marginal rate of zone 2 (0.14) and zone 3 (0.2397),
  - the two proportional rates (0.42, 0.45).

The quadratic coefficient of a linear-progressive zone follows from the
"Progressionsfaktor" construction: the marginal rate rises linearly from the
entry rate s_lo to the exit rate s_hi across the zone, so with
u = (x - E_lo) / 10000 and T = a*u**2 + b*u we have
  b = s_lo * 10000                         (marginal rate at u = 0)
  a = (s_hi * 10000 - b) / (2 * u_max)     (marginal rate s_hi at u = u_max).
The intercepts of the following zones follow from continuity of T.

This script derives all coefficients from the GETTSIM zone parameters and
verifies that, rounded to the published precision, they reproduce the literal
VZ 2026 coefficients fetched from gesetze-im-internet.de (§ 32a, Fassung
"ab dem Veranlagungszeitraum 2026", abgerufen 2026-07-09). This cross-check
validates the derivation method; the same method is used for VZ 2024 and 2025,
whose literal BGBl coefficients still have to be spot-checked (see reports).

Run: python params/derive_coefficients.py
"""

from __future__ import annotations

from fractions import Fraction as F

# Zone parameters, source: GETTSIM 1.2.1 einkommensteuertarif.yaml (BGBl refs below).
# gfb  = Grundfreibetrag (upper bound of the 0 zone)
# e2   = upper bound of zone 2 (first linear-progressive zone)
# e3   = upper bound of zone 3 (second linear-progressive zone)
# b    = start of the top proportional zone (0.45)
ZONE_PARAMS = {
    2024: dict(gfb=11784, e2=17005, e3=66760, b=277825,
               bgbl="Art. 1 G. v. 05.12.2024 BGBl. 2024 I Nr. 386"),
    2025: dict(gfb=12096, e2=17443, e3=68480, b=277825,
               bgbl="Art. 1 G. v. 30.12.2024 BGBl. 2024 Nr. 449"),
    2026: dict(gfb=12348, e2=17799, e3=69878, b=277825,
               bgbl="Art. 2 G. v. 30.12.2024 BGBl. 2024 Nr. 449"),
}

S1 = F(14, 100)      # entry marginal rate zone 2, § 32a Abs. 1
S2 = F(2397, 10000)  # entry marginal rate zone 3
T1 = F(42, 100)      # proportional rate zone 4
T2 = F(45, 100)      # proportional rate zone 5

# Literal published coefficients for VZ 2026 (gesetze-im-internet.de, 2026-07-09).
LITERAL_2026 = dict(a2=F("914.51"), a3=F("173.10"), c3=F("1034.87"),
                    d4=F("11135.63"), d5=F("19470.38"))


def derive(gfb, e2, e3, b, **_):
    ymax = F(e2 - gfb, 10000)
    a2 = (S2 * 10000 - S1 * 10000) / (2 * ymax)
    zmax = F(e3 - e2, 10000)
    a3 = (T1 * 10000 - S2 * 10000) / (2 * zmax)
    c3 = (a2 * ymax + S1 * 10000) * ymax      # tax at x = e2 (continuity)
    t3_e3 = (a3 * zmax + S2 * 10000) * zmax + c3
    d4 = T1 * e3 - t3_e3                       # continuity at x = e3
    d5 = d4 + (T2 - T1) * b                    # continuity at x = b
    return dict(a2=a2, a3=a3, c3=c3, d4=d4, d5=d5)


def q(frac, places=2):
    """Round a Fraction to `places` decimals, half away from zero (published format)."""
    scale = 10 ** places
    return round(float(frac), places), int(frac * scale + (F(1, 2) if frac >= 0 else F(-1, 2))) / scale


if __name__ == "__main__":
    for year, p in ZONE_PARAMS.items():
        d = derive(**p)
        print(f"VZ {year}  (GETTSIM/BGBl: {p['bgbl']})")
        print(f"  GFB={p['gfb']}  e2={p['e2']}  e3={p['e3']}  b={p['b']}")
        print(f"  a2={float(d['a2']):.4f}  a3={float(d['a3']):.4f}  c3={float(d['c3']):.4f}"
              f"  d4={float(d['d4']):.4f}  d5={float(d['d5']):.4f}")
        print(f"  published-format: a2={round(float(d['a2']),2)} a3={round(float(d['a3']),2)} "
              f"c3={round(float(d['c3']),2)} d4={round(float(d['d4']),2)} d5={round(float(d['d5']),2)}")

    print("\nCross-check VZ 2026 derived (2 dp) vs literal statute:")
    d26 = derive(**ZONE_PARAMS[2026])
    ok = True
    for k, lit in LITERAL_2026.items():
        got = round(float(d26[k]), 2)
        match = abs(F(str(got)) - lit) < F(1, 1000)
        ok = ok and match
        print(f"  {k}: derived={got}  literal={float(lit)}  {'OK' if match else 'MISMATCH'}")
    assert ok, "Derivation does not reproduce literal VZ 2026 coefficients"
    print("\nAll VZ 2026 coefficients reproduced exactly. Derivation method validated.")
