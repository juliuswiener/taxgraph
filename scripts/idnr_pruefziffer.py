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
    """Prueft eine vollstaendige 11-stellige IdNr gegen ihre eigene Pruefziffer.

    ACHTUNG: eine bestandene Pruefziffer heisst NICHT, dass ERiC die Nummer annimmt —
    dafuer muss zusaetzlich strukturell_gueltig() gelten. Siehe dort.
    """
    if len(idnr11) != 11 or not idnr11.isdigit():
        return False
    return pruefziffer(idnr11[:10]) == idnr11[10]


def strukturell_gueltig(idnr11: str) -> bool:
    """Die Aufbauregeln der IdNr NEBEN der Pruefziffer.

    Gelernt am 2026-08-19: 03165413965 hat eine korrekte Pruefziffer nach § 139b AO und wurde
    von ERiC trotzdem abgelehnt ("Ungueltige Identifikationsnummer"). Die Pruefziffer ist nur
    die halbe Regel. Zusaetzlich gilt:

      * die erste Ziffer ist nicht 0,
      * unter den ersten ZEHN Ziffern kommt GENAU EINE doppelt oder dreifach vor, alle
        uebrigen genau einmal.

    Die zweite Regel ist der Grund, warum man eine IdNr nicht einfach ausdenken kann: sie
    schliesst sowohl "alle Ziffern verschieden" als auch "mehrere Wiederholungen" aus.

    NICHT geprueft wird hier die dritte Feinheit (bei Dreifach-Vorkommen duerfen die drei nicht
    unmittelbar aufeinanderfolgen) — sie ist selten relevant und waere ohne einen Testfall, an
    dem man sie belegen kann, geraten.
    """
    if len(idnr11) != 11 or not idnr11.isdigit() or idnr11[0] == "0":
        return False
    from collections import Counter
    zaehler = Counter(idnr11[:10])
    mehrfach = [z for z, n in zaehler.items() if n > 1]
    return len(mehrfach) == 1 and zaehler[mehrfach[0]] in (2, 3)


if __name__ == "__main__":
    beispiel = "02476291358"  # bekanntes BZSt-Referenzbeispiel fuer die PRUEFZIFFER
    assert ist_gueltig(beispiel), f"Selbsttest fehlgeschlagen: {beispiel} sollte gueltig sein"
    print(f"Selbsttest ok: {beispiel} gueltig, Pruefziffer={pruefziffer(beispiel[:10])}")

    # Gegenprobe zur zweiten Regel: dieselbe Nummer ist STRUKTURELL unbrauchbar (fuehrende 0),
    # eine ERiC-taugliche muss beides erfuellen.
    assert not strukturell_gueltig(beispiel), "fuehrende 0 muesste strukturell auffallen"
    eric_tauglich = "86095742719"
    assert ist_gueltig(eric_tauglich) and strukturell_gueltig(eric_tauglich)
    print(f"Selbsttest ok: {eric_tauglich} ist pruefziffer- UND strukturgueltig")
