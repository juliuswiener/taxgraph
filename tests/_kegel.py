"""Der Pflicht-Kegel einer Scheibe: EIN Bauer fuer alle Tests.

WARUM DIESE DATEI EXISTIERT (2026-09-26). Sechsunddreissig Testdateien fuehrten den
Kegel als HANDGESCHRIEBENE Kopie. Als `agb_zwangslaeufig`/`agb_notwendig_angemessen`
in SCHEIBEN[...]["kegel"] aufgenommen wurden, lief keine dieser Kopien nach: jede
Datei sperrte auf `input_kegel_nicht_bestaetigt`, bevor ihr Messgegenstand ueberhaupt
griff — 297 Fehlschlaege aus EINER Ursache, ohne dass eine Zeile Code falsch war.
Der Defekt war nicht der Kegel-Eintrag, sondern die Kopie.

Der Bauer ist deshalb KEIN Ersatz fuer die Werte eines Tests, sondern sein Komplement:
Was der Test selbst setzt, gilt. Was er nicht setzt, fuellt der Bauer aus der Bindung.
Ein neues Kegel-Mitglied erscheint damit von selbst in jeder Datei — und kann keine
Testdatei mehr still rotmachen.

Bezugsgroesse ist `SCHEIBEN[scheibe]["kegel"]`, NICHT eine hier gepflegte Liste
([[geltungsbereich-ungleich-verwendung]]).

Werteherkunft je Feld: Aufrufer -> ABWESENHEITSWERT aus dem Typ (cent/int 0, bool die
Polaritaet des Namens, enum der `beispielwert`). Ausdruecklich NICHT der `beispielwert`
fuer bool/cent/int: er ist ILLUSTRATIV, nicht neutral. Gemessen 2026-09-26 widerspricht
er in ALLEN 15 bool-Kegel-Feldern dem Abwesenheitswert -- `kein_gewinn` traegt dort
`False` (= "ich habe Gewinneinkuenfte"), `agb_zwangslaeufig` `True` (= "die Regel
greift"). Ein Bauer, der damit fuellt, erfindet Sachverhalte und aendert die Rechnung
still; als A/B gemessen: 27 Tests in 13 Dateien rot.
"""
from __future__ import annotations

import glob
import importlib.util
import os
import sys

import yaml

HIER = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HIER)
PRODUKT = os.path.join(ROOT, "produkt")

for _sub in ("haut", "traverser", "store", "eingang", "mapping"):
    _p = os.path.join(PRODUKT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

_BINDUNG: dict | None = None


def _api_constants():
    spec = importlib.util.spec_from_file_location(
        "kegel_api_constants", os.path.join(PRODUKT, "haut", "api_constants.py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def bindung() -> dict:
    """Die geladene Bindungs-Tabelle, einmal je Sitzung."""
    global _BINDUNG
    if _BINDUNG is None:
        b: dict = {}
        for f in glob.glob(os.path.join(PRODUKT, "bindung", "bindung_*.yaml")):
            d = yaml.safe_load(open(f, encoding="utf-8")) or {}
            for e in d.get("bindungen", []):
                b[e["feld_id"]] = e
        _BINDUNG = b
    return _BINDUNG


def standardwert(feld_id: str, eintrag: dict | None = None) -> object:
    """Der ABWESENHEITS-Wert eines Kegel-Feldes — nicht der Beispielwert.

    Der Unterschied ist der ganze Punkt und war ein echter Fehler in der ersten
    Fassung: `beispielwert` ist ILLUSTRATIV, nicht neutral. `kein_vuv` traegt dort
    `False` — das BEHAUPTET "ich habe V+V-Einkuenfte". `vv_einnahmen` traegt
    `1200000`, also 12.000 EUR Einnahmen. Ein Bauer, der Fehlendes damit fuellt,
    ERFINDET Sachverhalte und veraendert die Rechnung still (gemessen 2026-09-26:
    test_ui_rechenweg.py kippte mit beispielwert-Defaults auf `flag_konsistenz_offen`,
    9 Faelle; mit Abwesenheitswerten gruen). Dieselbe Klasse wie
    [[nachbarwert-verraet-die-erfundene-zahl]] und [[geltungsbereich-ungleich-verwendung]].

    Deshalb: cent/int -> 0 (nichts), enum -> beispielwert, weil ein enum keinen
    Abwesenheitswert hat und geraten schlimmer waere als zu scheitern.

    Fuer bool entscheidet die POLARITAET des Namens, nicht ein pauschales True — der
    zweite Fehler derselben Bauart, eine Ebene tiefer: `kein_gewinn`/`kein_vuv` sind
    Abwesenheits-Flags, dort heisst True "habe ich nicht". `ep_eigenes_kfz` und
    `mit_anspruch_auf_zuschuss` sind das Gegenteil: True BEHAUPTET etwas ("ich fahre
    eigenen Wagen", "mir steht ein Zuschuss zu"). Ein pauschales True setzte
    `ep_eigenes_kfz=True` neben `ep_entfernung_km=0` und erzeugte damit genau den
    Widerspruch, den `flag_konsistenz_offen` meldet (gemessen 2026-09-26).
    """
    e = eintrag if eintrag is not None else (bindung().get(feld_id) or {})
    typ = e.get("typ")
    if typ == "bool":
        # ponytail: Polaritaet aus dem NAMEN, gilt nur weil gemessen (2026-09-26). Von 15
        # bool-Kegel-Feldern tragen 4 ein "kein_"-Praefix (dort ist True belegt: "habe ich
        # nicht"); die uebrigen 11 sind eine ANNAHME. Fuer 7 davon widerspricht sie der
        # Bindung (Heuristik False, beispielwert True: agb_*, dhf_*, ep_eigenes_kfz) --
        # die Bindung ist dort NICHT neutral, sondern illustrativ. Gemessen: die Bindung
        # als Fuellung macht 27 Tests in 13 Dateien rot, die Namensheuristik nicht. Ein
        # Feld ohne "kein_"-Praefix muesste streng genommen werfen statt zu raten; das
        # steht hier bewusst nicht, weil die Suite sonst an einer Annahme scheitert, die
        # der Aufrufer ohnehin immer selbst setzt (jedes der 11 hat ausdrueckliche
        # True-Overrides in tests/). Upgrade: je Feld ein `abwesenheitswert` in die
        # Bindung, dann raet der Name nicht mehr.
        return feld_id.startswith(("kein_", "keine_"))
    if typ in ("cent", "int"):
        return 0
    if typ == "enum" and "beispielwert" in e:
        return e["beispielwert"]
    raise ValueError(
        f"Kegel-Feld {feld_id!r} (typ={typ!r}) hat keinen Abwesenheitswert — der Test "
        "muss den Wert selbst setzen (einen enum-Wert zu raten waere erfunden)")


def kegel_fuer(scheibe: str, gesetzt: dict | None = None) -> list[tuple[str, object]]:
    """Der volle Pflicht-Kegel `scheibe` als (feld_id, wert)-Paare.

    `gesetzt` sind die Werte, auf die es dem Test ankommt; sie gewinnen. Jedes weitere
    Kegel-Mitglied kommt mit seinem Standardwert dazu, in der Reihenfolge des Kegels.
    Ein Feld, das der Test setzt und das NICHT im Kegel steht, wird angehaengt — sonst
    fiele eine bewusste Zusatzantwort still weg.
    """
    gesetzt = dict(gesetzt or {})
    ac = _api_constants()
    if scheibe not in ac.SCHEIBEN:
        raise KeyError(f"unbekannte Scheibe {scheibe!r}")
    b = bindung()
    raus: list[tuple[str, object]] = []
    gesehen = set()
    for feld_id in ac.SCHEIBEN[scheibe]["kegel"]:
        raus.append((feld_id, gesetzt.get(feld_id, standardwert(feld_id, b.get(feld_id)))))
        gesehen.add(feld_id)
    for feld_id, wert in gesetzt.items():
        if feld_id not in gesehen:
            raus.append((feld_id, wert))
    return raus


def fehlende_kegel_felder(scheibe: str, gesetzt) -> list[str]:
    """Welche Kegel-Mitglieder `gesetzt` NICHT enthaelt — die Ratsche fuer alte Handlisten.

    Eine Datei, die ihre Werte noch selbst fuehrt, kann damit pruefen, dass ihr nichts
    fehlt, statt es bei jeder Kegel-Aenderung neu zu merken.
    """
    ac = _api_constants()
    if isinstance(gesetzt, dict):
        namen = set(gesetzt)
    else:
        # Paare (feld_id, wert) ODER blosse feld_ids -- beides ist zulaessig.
        namen = {e[0] if isinstance(e, (tuple, list)) else e for e in gesetzt}
    return [f for f in ac.SCHEIBEN[scheibe]["kegel"] if f not in namen]
