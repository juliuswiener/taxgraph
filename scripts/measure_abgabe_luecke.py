#!/usr/bin/env python3
"""Eine einzelne Abgabe-Luecke gegen das amtliche checkESt-Plugin messen — schichtweise.

WOZU. Die vier bereits geschlossenen Luecken (12ec7a3 Realsplitting, e814fd0 Pflege,
af41d6a § 35c, e9e61bf Anlage V) haben alle dasselbe gezeigt: ERiC gibt seine Anforderungen
NICHT auf einmal preis. Wer nach der ersten gruenen Meldung aufhoert, haelt einen halben Bau
fuer fertig. Anlage V brauchte fuenf Schichten, § 35c drei, der Pflege-Pauschbetrag drei.

Dieses Skript macht daraus einen wiederholbaren Handgriff statt einer Handarbeit pro Luecke:
es setzt eine Feldgruppe auf den Ratschen-Basisfall und meldet rc plus die Beanstandungen im
Klartext. Man ruft es nach jeder gebauten Schicht erneut auf und sieht, was ERiC als NAECHSTES
verlangt.

NUR MESSEN. Schreibt keine Repo-Datei, veraendert keinen Store, sendet nichts (ERIC_VALIDIERE
ohne ERIC_SENDE, das Plugin laeuft lokal). Gibt NIE Hersteller-ID, Steuernummer oder IdNr aus.

    set -a; . ./.env; set +a
    python3 scripts/measure_abgabe_luecke.py --felder ep_arbeitstage=220 ep_entfernung_km=42
    python3 scripts/measure_abgabe_luecke.py --gruppe ep          # vordefinierte Feldgruppe

Der Messweg ist NICHT neu gebaut, sondern 1:1 der des offiziellen Gates
(tests/test_checkest_feldmatrix.py::_scharf). Das ist Absicht: eine eigene Fassung ging dort
einmal direkt store -> est_mapping.deklariere und uebersprang die Ring-Injektionen; ein Bau,
der nur in _mit_ring_werten sitzt, waere dabei unsichtbar gruen geblieben.
"""
from __future__ import annotations

import argparse
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "tests"))

import test_checkest_feldmatrix as TCF   # noqa: E402  -- liefert _scharf/_fall_einzel 1:1

# Vordefinierte Gruppen: die Felder, die eine Luecke ueberhaupt erst ausloesen. Ohne sie
# kommt die Anlage gar nicht in die Erklaerung und ERiC hat nichts zu beanstanden — die
# Messung waere dann still gruen und wertlos.
GRUPPEN: dict[str, dict[str, object]] = {
    "ep":  {"ep_arbeitstage": 220, "ep_entfernung_km": 42},
    "dhf": {},        # wird nach der Recherche gefuellt
    "ausb": {},
    "rentner_hinterbl": {},
}


def _wert(roh: str) -> object:
    """Kommandozeilen-Text in den Typ, den die Bindung erwartet.

    NICHT kosmetisch. Eine erste Fassung wandelte nur Ziffern in int und liess alles andere
    als String stehen — `rentner_hinterbliebenenbezuege=true` ging damit als der STRING "true"
    in ein bool-Feld, und ERiC beanstandete "Unzulaessiges Zeichen: Es darf nur '1' (fuer Ja)
    eingetragen werden". Das sah aus wie ein Befund ueber das Produkt und war einer ueber das
    Messwerkzeug. Genau die Verwechslung, die dieses Skript verhindern soll.
    """
    n = roh.lower()
    if n in ("true", "ja", "wahr"):
        return True
    if n in ("false", "nein", "falsch"):
        return False
    if roh.lstrip("-").isdigit():
        return int(roh)
    return roh


def _store_mit(felder: dict[str, object]):
    """Ratschen-Basisfall plus mehrere Felder.

    TCF._mit setzt genau EIN Feld; fuer eine Luecke braucht es die ganze Gruppe, sonst
    springt der Container nicht an. Der Store ist fail-closed gegen das Ueberschreiben eines
    aktiven Events (store.py:232), darum wird ein schon gesetztes Feld ersetzt statt doppelt
    angelegt — dieselbe Logik wie in TCF._mit, nur ueber mehrere Felder.
    """
    import store as ST
    s = TCF._fall_einzel()
    for feld_id, wert in felder.items():
        aktiv = None
        for e in reversed(s.get("events") or []):
            if e.get("feld_id") == feld_id and not e.get("ersetzt_durch"):
                aktiv = e["event_id"]
                break
        if aktiv:
            ST.append_event(s, feld_id=feld_id, wert=wert, zustand="bestaetigt",
                            herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft",
                                      "haftung": "nutzer"},
                            schreiber="laie",
                            signal={"signal_1": {"typ": "laie_eingabe"},
                                    "signal_2": "laie_bestaetigt"},
                            ersetzt=aktiv)
        else:
            TCF._b(s, feld_id, wert)
    return s


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--gruppe", choices=sorted(GRUPPEN), help="vordefinierte Feldgruppe")
    p.add_argument("--felder", nargs="*", default=[], metavar="feld_id=wert",
                   help="zusaetzliche Felder, ergaenzen/ueberschreiben die Gruppe")
    p.add_argument("--basis", action="store_true",
                   help="nur den Basisfall messen (Gegenprobe: was ist OHNE die Gruppe rot?)")
    a = p.parse_args()

    if not TCF._HID:
        return int(bool(sys.stderr.write(
            "ELSTER_HERSTELLER_ID fehlt — 'set -a; . ./.env; set +a' vor dem Aufruf.\n"
            "OHNE sie faellt die Messung auf die credential-freie GESPERRT-Grenze zurueck\n"
            "und meldet etwas, das wie ein Ergebnis aussieht, aber keins ist.\n")) or 2)
    eric = os.environ.get("ERIC_DIR", os.path.expanduser("~/02_Software/eric"))
    if not os.path.isdir(eric):
        return int(bool(sys.stderr.write(f"$ERIC_DIR fehlt oder ist kein Verzeichnis: {eric}\n")) or 2)

    felder: dict[str, object] = {}
    if a.gruppe:
        felder.update(GRUPPEN[a.gruppe])
    for paar in a.felder:
        if "=" not in paar:
            return int(bool(sys.stderr.write(f"kein feld_id=wert: {paar!r}\n")) or 2)
        k, _, v = paar.partition("=")
        felder[k.strip()] = _wert(v.strip())
    if a.basis:
        felder = {}

    store = _store_mit(felder)
    rc, texte = TCF._scharf(store)

    print(f"Felder gesetzt: {', '.join(sorted(felder)) or '(nur Basisfall)'}")
    print(f"rc={rc}  Beanstandungen: {len(texte)}")
    for i, t in enumerate(texte, 1):
        print(f"  {i:2}. {t}")
    if rc == 0 and not texte:
        print("\nrc=0 und leerer Puffer — diese Schicht ist durch.")
        print("ACHTUNG: das gilt fuer GENAU diese Feldgruppe. Eine Anlage kann mehrere")
        print("Ausloeser haben (§ 35c: Pflegegrad-Staffel UND Merkzeichen H fuehrten")
        print("getrennt hinein). Die andere Haelfte ist damit nicht gemessen.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
