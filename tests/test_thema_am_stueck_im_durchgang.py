"""Ein Thema bleibt AUCH IM DURCHGANG am Stück — nicht nur in einem einzelnen Aufruf.

Julius, 2026-08-27, aus einem protokollierten echten Durchgang:

    Kirchensteuer   p51a (#116) -> p10_1_4 (#117,#118) -> p51a (#119)
    Arbeitsmittel   p7_1 (#44-46) -> p9_1_3_nr6_7 (#65-67) -> p7_1 (#105-107)
    KV/PV           p10_1_3_3a in DREI Stücken: #68/69, #120, #130-134
    § 35a           in zwei Stücken: #47-63 und #122

WARUM DIE BESTANDSTESTS DAS NICHT SEHEN — und das ist der eigentliche Punkt dieser Datei:
`tests/test_eingangsfrage_zuerst.py::test_die_queue_springt_nicht_zwischen_themen_hin_und_her`
misst EINEN Aufruf von `naechste_fragen` auf leerem Store und findet dort das theoretische Minimum
(61 Wechsel bei 62 Themen). Diese Messung ist richtig und bleibt es. Nur erlebt der Nutzer nie
eine einzelne Queue: `naechste_fragen` läuft nach JEDER Antwort neu, und der Fehler entsteht
zwischen zwei Aufrufen. Er ist in keinem Einzelbild sichtbar.

GEMESSEN an einem simulierten Volldurchgang (Kopf der Queue beantworten, neu rechnen, 288 Fragen):

    vorher    116 Themenwechsel bei 74 Themen, 28 zerschnittene Themen, 43 überzählige Blöcke
    nachher    72 Themenwechsel bei 73 Themen,  0 zerschnittene Themen,  0 überzählige Blöcke

URSACHE, gemessen und nicht geraten: von den 43 Abbrüchen mitten im Thema entstanden **43 durch
Rangverlust und 0 dadurch, dass eine Frage noch nicht in der Queue stand.** Die Reihenfolge der
Themen entsteht aus der Position des BESTEN Feldes je Thema; genau dieses Feld beantwortet der
Nutzer zuerst, danach fällt das Thema auf den Rang seiner schwächeren Felder zurück. Die
abgebrochenen Fragen standen jedes Mal direkt hinter der beantworteten und rutschten auf Platz 44,
91, 126, 165.

NULL LLM.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/traverser", "produkt/store"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import store as ST      # noqa: E402
import traverser as TR  # noqa: E402

LAIE = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}
KLICK = {"signal_1": None, "signal_2": "klick"}


def _antwort(fid: str, e: dict):
    """Eine plausible Antwort je Bindungstyp — deterministisch, damit der Durchgang wiederholbar ist.

    `kein_*`/`keine_*` heisst „nichts davon", wird also mit False beantwortet: der simulierte Fall
    HAT alles. Das ist Absicht — je mehr Themen offen bleiben, desto mehr Gelegenheit hat die
    Sortierung, eines davon zu zerschneiden.
    """
    typ = e.get("typ")
    if typ == "bool":
        return not fid.startswith(("kein_", "keine_"))
    if typ == "cent":
        return 100000
    if typ == "int":
        return 2
    if typ == "enum":
        return (e.get("enum_werte") or ["a"])[0]
    beispiel = e.get("beispielwert")
    if isinstance(beispiel, str):
        return beispiel                       # erfüllt `muster`, wo eines deklariert ist
    return "01.01.1990" if typ == "datum" else "Musterwert"


def _durchgang(bindung: dict, grenze: int = 400) -> list[str]:
    """Der Ablauf, den ein Mensch erlebt: erste Frage der Queue beantworten, Queue neu rechnen.

    Felder, deren Beispielwert die eigene Formatprüfung nicht besteht, werden übersprungen statt
    den Test rot zu machen — das wäre ein Bindungsfehler und gehört nicht in eine Messung der
    Reihenfolge (`test_jede_uebersprungene_frage_ist_benannt` unten hält die Liste trotzdem kurz).
    """
    s = ST.leerer_store(veranlagungszeitraum=2025, fall_id="durchgang")
    s["scheibe"] = "gesamt"
    gestellt: list[str] = []
    blockiert: set[str] = set()
    while len(gestellt) < grenze:
        offen = [f for f in TR.naechste_fragen(s, bindung) if f not in blockiert]
        if not offen:
            break
        fid = offen[0]
        try:
            ST.append_event(s, feld_id=fid, wert=_antwort(fid, bindung[fid]),
                            zustand="bestaetigt", herkunft=LAIE, schreiber="ui:laie",
                            signal=KLICK, bindung=bindung)
        except ValueError:
            blockiert.add(fid)
            continue
        gestellt.append(fid)
    return gestellt


def _bloecke(gestellt: list[str], bindung: dict) -> list[tuple[str, list[str]]]:
    """Zusammenhängende Blöcke gleichen Themas, in der Reihenfolge des Durchgangs."""
    out: list[tuple[str, list[str]]] = []
    for f in gestellt:
        thema = (bindung[f].get("quelle") or {}).get("regel_id") or ""
        if out and out[-1][0] == thema:
            out[-1][1].append(f)
        else:
            out.append((thema, [f]))
    return out


@pytest.fixture(scope="module")
def durchgang():
    b = TR.lade_bindung()
    return _durchgang(b), b


def test_kein_thema_wird_zerschnitten(durchgang):
    """DIE INVARIANTE. Jedes Thema kommt im ganzen Durchgang in GENAU EINEM Stück.

    Vor dem Eingriff waren es 28 zerschnittene Themen in 43 Stücken zu viel. Eine Ratsche („nicht
    mehr als vorher") wäre hier zu schwach: sie hätte den Zwischenstand durchgelassen, bei dem
    § 35a nur noch in zwei statt drei Stücken kam — und zwei Stücke sind genau das, worüber Julius
    sich beschwert hat."""
    gestellt, b = durchgang
    assert len(gestellt) > 150, f"Nur {len(gestellt)} Fragen gestellt — die Messung greift nicht."
    bloecke = _bloecke(gestellt, b)
    themen = {t for t, _ in bloecke}
    mehrfach: dict[str, list[str]] = {}
    lauf = 0
    for thema, felder in bloecke:
        if sum(1 for t, _ in bloecke if t == thema) > 1:
            mehrfach.setdefault(thema, []).append(f"#{lauf + 1}-{lauf + len(felder)}")
        lauf += len(felder)
    assert not mehrfach, (
        f"{len(mehrfach)} Themen werden zerschnitten — der Nutzer wird zwischen ihnen hin- und "
        f"hergeschickt:\n  " + "\n  ".join(f"{t}: {', '.join(st)}" for t, st in mehrfach.items()))
    assert len(bloecke) - 1 == len(themen) - 1, "Blockzählung und Themenzählung passen nicht."


def test_die_faelle_aus_julius_durchgang_namentlich(durchgang):
    """Namentlich, damit ein Rückbau auffällt und nicht bloss die Gesamtzahl wieder steigt.

    Diese vier Themen hat Julius im Protokoll vom 2026-08-27 einzeln benannt. Ein Sweep über alle
    Themen kann grün bleiben, während ausgerechnet sie wieder auseinanderfallen — dann sagt die
    Zahl, es sei besser geworden, und der gemeldete Fall ist trotzdem offen."""
    gestellt, b = durchgang
    for thema in ("p35a_2_3_haushaltsnahe", "p10_1_3_3a_kv_pv", "p51a_kirchensteuer",
                  "p7_1_lineare_afa"):
        plaetze = [i for i, f in enumerate(gestellt)
                   if (b[f].get("quelle") or {}).get("regel_id") == thema]
        if not plaetze:
            continue          # Thema im simulierten Fall ausgeschlossen — sagt über Ordnung nichts
        spanne = plaetze[-1] - plaetze[0] + 1
        assert spanne == len(plaetze), (
            f"{thema}: {len(plaetze)} Fragen über {spanne} Plätze verteilt, dazwischen stehen "
            f"{spanne - len(plaetze)} fremde Fragen (Plätze "
            f"#{plaetze[0] + 1}-#{plaetze[-1] + 1}).")


def test_ein_angefangenes_thema_bleibt_vorn_auch_gegen_ein_schwereres(durchgang):
    """Der Mechanismus einzeln, ohne den ganzen Durchgang — damit im Fehlerfall klar ist, WAS kaputt
    ist und nicht nur, DASS die Gesamtzahl steigt.

    AUSDRÜCKLICH NICHT AM ERÖFFNUNGSTHEMA GEMESSEN, und das ist der Unterschied zwischen einem Test
    und einem Stellvertreter: die Themen aus `themen_zuerst` stehen ohnehin vorn, ganz ohne diese
    Regel. Wer den Mechanismus an `veranlagung` misst, bekommt grün, auch wenn er ausgebaut ist —
    genau so war dieser Test zuerst geschrieben, und die Mutationsprobe hat es gezeigt.

    Gemessen wird deshalb am ERSTEN Thema, das NICHT zum Einstieg gehört: dort und nur dort
    entscheidet sich, ob eine begonnene Antwortstrecke weiterläuft oder von einem schwereren Thema
    verdrängt wird."""
    _, b = durchgang
    einstieg = set(TR.lade_themen_zuerst())
    s = ST.leerer_store(veranlagungszeitraum=2025, fall_id="klebrig")

    for _schritt in range(400):
        queue = TR.naechste_fragen(s, b)
        assert queue, "Queue leer, bevor ein Thema ausserhalb des Einstiegs erreicht war."
        kopf = queue[0]
        thema = (b[kopf].get("quelle") or {}).get("regel_id")
        rest_im_thema = [f for f in queue[1:]
                         if (b[f].get("quelle") or {}).get("regel_id") == thema]
        ST.append_event(s, feld_id=kopf, wert=_antwort(kopf, b[kopf]), zustand="bestaetigt",
                        herkunft=LAIE, schreiber="ui:laie", signal=KLICK, bindung=b)
        if thema in einstieg or not rest_im_thema:
            continue
        weiter = TR.naechste_fragen(s, b)
        assert weiter[0] == rest_im_thema[0], (
            f"Nach der Antwort auf {kopf} ({thema}) steht {weiter[0]!r} vorn statt der nächsten "
            f"Frage desselben Themas ({rest_im_thema[0]!r}). Das Thema hatte noch "
            f"{len(rest_im_thema)} offene Fragen und wurde weggerankt — genau der Abbruch, den "
            f"Julius am 2026-08-27 43-mal im Durchgang hatte.")
        return
    pytest.fail("Kein Thema ausserhalb des Einstiegs erreicht — die Messung greift nicht.")


def test_ein_vorlaeufiger_vorschlag_faengt_kein_thema_an(durchgang):
    """`_angefangene_themen` zählt nur BESTÄTIGT — wie alles andere im Traverser auch.

    Sonst zöge ein vorläufiger KI-Vorschlag ein ganzes Thema an den Anfang, das der Nutzer nie
    gesehen hat: er hätte plötzlich zwölf Fragen zur Zweitwohnung vor sich, weil die KI in einem
    Beleg eine Adresse gefunden hat."""
    _, b = durchgang
    kandidaten = [f for f, e in b.items()
                  if e.get("askable") and (e.get("quelle") or {}).get("regel_id")
                  == "p9_1_3_nr5_doppelte_haushaltsfuehrung"]
    assert kandidaten, "Vorbedingung: das Zweitwohnungs-Thema muss askable Felder haben."
    s = ST.leerer_store(veranlagungszeitraum=2025, fall_id="vorlaeufig")
    ohne = TR.naechste_fragen(s, b)
    feld = sorted(kandidaten)[0]
    ST.append_event(s, feld_id=feld, wert=_antwort(feld, b[feld]), zustand="vorlaeufig",
                    herkunft=LAIE, schreiber="ki:vorschlag", signal=KLICK, bindung=b)
    mit = TR.naechste_fragen(s, b)

    # DIE GANZE QUEUE, nicht nur ihr Kopf. Der Kopf gehört zum Eröffnungsthema und bleibt auch dann
    # stehen, wenn ein Vorschlag ein Thema fälschlich nach vorn zieht — die Mutationsprobe hat
    # diesen Test mit einem Kopf-Vergleich GRÜN gelassen, obwohl die Regel ausgebaut war.
    assert mit == ohne, (
        f"Ein vorläufiger Vorschlag zu {feld!r} hat die Reihenfolge verändert: das Thema steht "
        f"jetzt auf Platz {mit.index(feld) + 1} statt {ohne.index(feld) + 1}. Es gilt als "
        f"angefangen, obwohl der Nutzer nichts beantwortet hat — ein Beleg-Fund der KI zöge ihm "
        f"damit zwölf Fragen zur Zweitwohnung an den Anfang.")


def test_die_bestandsordnung_gilt_im_durchgang_weiter(durchgang):
    """Die Klebrigkeit darf die vorhandenen Regeln nicht aushebeln, sondern nur ergänzen.

    Drei davon sind im Durchgang prüfbar und alle drei sind an einem echten Befund gebaut worden:
    die Eingangsfrage eröffnet ihr Thema (2026-08-21), die Ableitungsquelle steht vor ihrem Ziel
    (2026-08-26), und das Zählfeld steht vor den Feldern, die aus ihm entstehen (2026-08-27).

    Geprüft wird hier gegen den DURCHGANG, nicht gegen eine einzelne Queue — dieselbe Verschiebung,
    die ein Thema zerschneidet, könnte auch eine Quelle hinter ihr Ziel schieben, und in einem
    Einzelbild wäre davon nichts zu sehen."""
    gestellt, b = durchgang
    platz = {f: i for i, f in enumerate(gestellt)}

    zu_spaet = []
    for thema, felder in _bloecke(gestellt, b):
        eingang = [f for f in felder if b[f].get("eingangsfrage")]
        if eingang and felder[0] != eingang[0]:
            zu_spaet.append(f"{thema}: beginnt mit {felder[0]}, Eingangsfrage ist {eingang[0]}")
    assert not zu_spaet, "Eingangsfrage eröffnet ihr Thema nicht:\n  " + "\n  ".join(zu_spaet)

    quelle_zu_spaet = []
    for ziel, e in b.items():
        regel = e.get("ableitung")
        if regel and ziel in platz and regel["aus"] in platz and platz[regel["aus"]] > platz[ziel]:
            quelle_zu_spaet.append(
                f"{ziel} (#{platz[ziel] + 1}) wird aus {regel['aus']} (#{platz[regel['aus']] + 1}) "
                f"berechnet — die Quelle kommt zu spät, die Ableitung greift nie.")
    assert not quelle_zu_spaet, "Ableitung vor ihrer Quelle:\n  " + "\n  ".join(quelle_zu_spaet)

    verdreht = []
    for gruppe, eintrag in TR.lade_instanz_gruppen().items():
        zaehl = eintrag["anzahl_feld"]
        instanzen = [platz[f] for f, e in b.items()
                     if e.get("instanz_gruppe") == gruppe and f != zaehl and f in platz]
        if not instanzen or zaehl not in platz:
            continue
        if platz[zaehl] > min(instanzen):
            verdreht.append(f"{gruppe}: Zählfeld {zaehl} auf #{platz[zaehl] + 1}, erstes "
                            f"Instanzfeld schon auf #{min(instanzen) + 1}")
    assert not verdreht, "Zählfeld hinter seinen Instanzfeldern:\n  " + "\n  ".join(verdreht)
