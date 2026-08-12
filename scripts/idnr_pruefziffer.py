"""Pruefziffer der steuerlichen Identifikationsnummer (IdNr, § 139b AO).

Algorithmus: ISO 7064 MOD 11,10, wie in Anlage 2 zu § 139c AO vorgeschrieben.
11-stellige IdNr = 10 Basisziffern + 1 Pruefziffer.

Zentral halten, nicht ein drittes Mal bauen: schon fuer person_b_idnr
(bindung_an_gesamt.yaml) und kind_idnr (bindung_kap_vv_familie.yaml) als
Beispielwert-Quelle genutzt.

Selbsttest gegen das bekannte BZSt-Referenzbeispiel: python scripts/idnr_pruefziffer.py
"""


def pruefziffer(basis10: str) -> str:
    """Berechnet die 11. Ziffer (Pruefziffer) zu 10 Basisziffern."""
    assert len(basis10) == 10 and basis10.isdigit(), "basis10 muss genau 10 Ziffern sein"
    produkt = 10
    for ch in basis10:
        d = int(ch)
        summe = (produkt + d) % 10
        if summe == 0:
            summe = 10
        produkt = (summe * 2) % 11
    pz = (11 - produkt) % 10
    return str(pz)


def ist_gueltig(idnr11: str) -> bool:
    """Prueft eine vollstaendige 11-stellige IdNr gegen ihre eigene Pruefziffer."""
    if len(idnr11) != 11 or not idnr11.isdigit():
        return False
    return pruefziffer(idnr11[:10]) == idnr11[10]


if __name__ == "__main__":
    beispiel = "02476291358"  # bekanntes BZSt-Referenzbeispiel
    assert ist_gueltig(beispiel), f"Selbsttest fehlgeschlagen: {beispiel} sollte gueltig sein"
    print(f"Selbsttest ok: {beispiel} gueltig, Pruefziffer={pruefziffer(beispiel[:10])}")
