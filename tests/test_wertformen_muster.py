"""Gate: das Messwerkzeug `tools/parity/wertformen.py` prueft `muster` so streng wie das Speichern.

Ticket `wertformen-messwerkzeug-prueft-muster-nur-am-anfang`: `abweichung` nutzt `re.match`.
`re.match` verankert nur am ANFANG. Mit einem `$`-Muster, das CPython vor einem finalen
Zeilenumbruch matchen laesst, geht ein Wert `"<gueltig>\\n"` als passend durch — obwohl
`store._typ_konform` ihn beim Speichern abweist. Eine Bestandsmessung faellt damit zu guenstig aus.

Geprueft wird an der Funktion `abweichung`, nicht am Etikett der Tabellenausgabe: dieselbe
Stelle, die `zaehle()` aufruft, ohne Store und ohne Golden-Dateien.
"""
from __future__ import annotations

import os
import re
import sys

import pytest

_HERE = os.path.dirname(os.path.abspath(__file__))
_ROOT = os.path.dirname(_HERE)
_PARITY = os.path.join(_ROOT, "tools", "parity")
if _PARITY not in sys.path:
    sys.path.insert(0, _PARITY)

wertformen = pytest.importorskip("wertformen")

# Ein echtes Bindungsmuster: `kind_betreuung_zeitraum` (typ=text) aus bindung_an_gesamt.yaml.
MUSTER = r"^\d{2}\.\d{2}-\d{2}\.\d{2}$"
GUELTIG = "01.01-31.12"
EINTRAG = {"typ": "text", "muster": MUSTER}

# Kandidaten fuer den Geschwistertest: je Muster der Bindung ein Wert, der es erfuellt.
# Die Bindung traegt Zeitraeume (TT.MM-TT.MM), Daten (TT.MM.JJJJ), IdNr (11 Ziffern), PLZ,
# BIC, Hausnummer und `dhf_bestanden_bis` (TT.MM. ohne Jahr). Nur ein Kandidat, der
# `re.fullmatch(muster, ...)` besteht, wird benutzt.
KANDIDATEN = (
    "01.01-31.12",       # Zeitraum TT.MM-TT.MM
    "05.05.1955",        # Datum TT.MM.JJJJ
    "12345678901",       # IdNr, 11 Ziffern
    "10115",             # PLZ
    "COBADEFFXXX",       # BIC
    "12a",               # Hausnummer mit Buchstabe
    "31.12.",            # dhf_bestanden_bis: TT.MM. ohne Jahr
    "12",                # Hausnummer, reine Ziffern
)


def test_re_match_ist_hier_nicht_genug():
    """Warum der Test unten rot werden MUSS: `re.match` gegen `re.fullmatch` am selben Wert.

    Nicht die zu testende Stelle, sondern der Beweis, dass die beiden Funktionen sich hier
    ueberhaupt unterscheiden. Ohne diesen Unterschied waere der Test unten zahnlos.
    """
    wert = GUELTIG + "\n"
    assert re.match(MUSTER, wert) is not None, "re.match verankert nur am Anfang"
    assert re.fullmatch(MUSTER, wert) is None, "re.fullmatch prueft den ganzen String"


def test_gueltiger_wert_ist_keine_abweichung():
    """Gegenprobe: der Test unten misst nicht einfach 'immer Abweichung'."""
    assert wertformen.abweichung(GUELTIG, "text", EINTRAG) is None


def test_wert_mit_zeilenumbruch_ist_musterverstoss():
    """DER ROT-TEST: ein gueltiger Wert mit angehaengtem Zeilenumbruch verletzt das Muster.

    `store._typ_konform` weist ihn beim Speichern ab; das Messwerkzeug muss ihn ebenso zaehlen.
    """
    wert = GUELTIG + "\n"
    assert wertformen.abweichung(wert, "text", EINTRAG) == "string verletzt muster auf text"


def test_geschwister_alle_muster_eintraege():
    """Dieselbe Pruefung fuer JEDEN Muster-Eintrag der echten Bindung, nicht nur einen.

    Der gueltige Wert wird NICHT fest verdrahtet, sondern gegen `re.fullmatch(muster, ...)`
    gesucht — das ist derselbe Massstab, den das Speichern anlegt. Ein fest verdrahtetes
    Format waere hier falsch: die Bindung traegt Zeitraeume, Daten, IdNr, PLZ, BIC und
    Hausnummern, und ein Test, der nur eines kennt, prueft die anderen still nicht mit.

    Geprueft wird die Zusicherung des Tickets, nicht das Etikett: ein Wert, der das Muster
    erfuellt, geht durch; derselbe Wert mit angehaengtem Zeilenumbruch nicht. WELCHES Etikett
    dabei faellt, haengt vom Zweig ab (`datum` laeuft durch `_typ_konform`, `text` durch den
    Muster-Zweig) und ist nicht die Aussage.
    """
    bindung = wertformen.TR.lade_bindung()
    mit_muster = {f: e for f, e in bindung.items() if e.get("muster")}
    assert mit_muster, "keine muster-Eintraege in der Bindung — Test waere zahnlos"
    geprueft = 0
    for fid, eintrag in mit_muster.items():
        muster = eintrag["muster"]
        # Kandidaten, die in diesem Repo die vorkommenden Formate abdecken.
        gueltig = next(
            (k for k in KANDIDATEN if re.fullmatch(muster, k) is not None), None
        )
        if gueltig is None:
            continue  # kein Kandidat fuer dieses Muster — unten wird die Zahl geprueft
        # Der Kandidat muss den ganzen Weg nehmen, sonst misst der Test das Falsche.
        assert wertformen.abweichung(gueltig, wertformen.form(gueltig), eintrag) is None, fid
        mit_umbruch = gueltig + "\n"
        assert wertformen.abweichung(
            mit_umbruch, wertformen.form(mit_umbruch), eintrag
        ) is not None, f"{fid}: '{{gueltig}}\\n' galt als passend (Muster {muster!r})"
        geprueft += 1
    assert geprueft == len(mit_muster), (
        f"nur {geprueft} von {len(mit_muster)} Muster-Eintraegen geprueft — "
        f"KANDIDATEN um die fehlenden Formate ergaenzen"
    )
