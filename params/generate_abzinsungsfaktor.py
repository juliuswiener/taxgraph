"""Generiert die Abzinsungsfaktor-Tabelle (§ 6 Abs. 1 Nr. 3a Buchst. e EStG).

Deterministische Ableitung aus der gefrorenen Norm-Konstante 5,5 %:
    faktor(n) = 1 / 1,055^n   (n = Restlaufzeit in ganzen Jahren)
kaufmaennisch auf 3 Dezimalen gerundet (ROUND_HALF_UP).

Stuetzwert-Verifikation gegen BMF v. 26.05.2005 (BStBl I S. 699, Tabelle 2):
RLZ 1 -> 0,948; RLZ 10 -> 0,585; RLZ 19 -> 0,362 (siehe tests/test_abzinsungsfaktor.py).

Gueltigkeit (4. Corona-Steuerhilfegesetz): die Tabelle gilt NUR fuer RUECKSTELLUNGEN
(§ 6 Abs. 1 Nr. 3a Buchst. e). Der Verbindlichkeiten-Teil (§ 6 Nr. 3 a. F., BMF v.
26.05.2005) ist obsolet ab Wirtschaftsjahr-Ende > 31.12.2022.

Aufruf: python3 params/generate_abzinsungsfaktor.py  -> schreibt
params/kohorten/abzinsungsfaktor_5komma5_p6.yaml (Restlaufzeit 1..50).
"""
from decimal import Decimal, ROUND_HALF_UP
import os

ZINSSATZ = Decimal("1.055")
MAX_RESTLAUFZEIT = 50
OUT = os.path.join(os.path.dirname(__file__), "kohorten", "abzinsungsfaktor_5komma5_p6.yaml")


def faktor(n: int) -> Decimal:
    """1 / 1,055^n, kaufmaennisch 3 Dezimalen."""
    return (Decimal(1) / (ZINSSATZ ** n)).quantize(Decimal("0.001"), rounding=ROUND_HALF_UP)


def build_yaml() -> str:
    zeilen = [
        "# § 6 Abs. 1 Nr. 3a Buchst. e EStG - Rueckstellungs-Abzinsungsfaktoren (5,5 %).",
        "# Restlaufzeit in ganzen Jahren -> Barwertfaktor. NICHT vz-versioniert (mathematische",
        "# Konstante). Deterministisch abgeleitet: faktor(n) = 1/1,055^n, kaufmaennisch 3 Dezimalen.",
        "# Regeneriert via params/generate_abzinsungsfaktor.py; getestet tests/test_abzinsungsfaktor.py.",
        "# GUELTIGKEIT (4. Corona-StHG): gilt NUR Rueckstellungen (Nr. 3a e). Verbindlichkeiten-Abzinsung",
        "# (Nr. 3 a. F., BMF v. 26.05.2005) obsolet ab WJ-Ende > 31.12.2022.",
        "parameter: abzinsungsfaktor_rueckstellung_5komma5",
        "authority: gesetz",
        "redistributable: true",
        "rechtsquelle: {gesetz: EStG, paragraph: '6', absatz: '1', nummer: '3a', buchstabe: 'e'}",
        "datenquelle: 'Deterministische Ableitung 1/1,055^n aus der Norm-Konstante 5,5 % "
        "(sources/gesetze-im-internet/estg_p6_2026-07-14.txt); Stuetzwerte gegen BMF v. 26.05.2005 "
        "(BStBl I S. 699, Tabelle 2): RLZ 1->0,948 / 10->0,585 / 19->0,362.'",
        "einheit: {faktor: faktor}",
        "",
        "# Schluessel = Restlaufzeit am Bilanzstichtag in ganzen Jahren.",
        "kohorten:",
    ]
    for n in range(1, MAX_RESTLAUFZEIT + 1):
        zeilen.append(f"  {n}: {{faktor: {faktor(n)}}}")
    return "\n".join(zeilen) + "\n"


if __name__ == "__main__":
    with open(OUT, "w", encoding="utf-8") as f:
        f.write(build_yaml())
    print(f"geschrieben: {OUT} (Restlaufzeit 1..{MAX_RESTLAUFZEIT})")
