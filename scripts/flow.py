#!/usr/bin/env python3
"""Den Fluss-Mitschnitt lesbar machen: was gefragt wurde, was geantwortet, was die KI dazwischen tat.

    python scripts/flow.py                 # letzter Fall
    python scripts/flow.py <fall_id>       # ein bestimmter
    python scripts/flow.py --faelle        # welche Fälle stehen im Log

Aufgezeichnet wird nur mit `TAXGRAPH_FLOW=1` (s. produkt/haut/flow.py). Ohne die Variable ist die
Datei nicht da, und dieses Skript sagt das, statt eine leere Auswertung zu zeigen — „keine Befunde"
und „nichts gemessen" sehen sonst gleich aus.
"""
from __future__ import annotations

import json
import os
import sys

PFAD = os.path.join(
    os.environ.get("XDG_DATA_HOME") or os.path.expanduser("~/.local/share"),
    "taxgraph", "faelle", "flow.jsonl")


def lies() -> list[dict]:
    if not os.path.exists(PFAD):
        print(f"Kein Mitschnitt unter {PFAD}.\n"
              f"Der Server muss mit TAXGRAPH_FLOW=1 laufen:\n"
              f"    TAXGRAPH_FLOW=1 python produkt/haut/server.py 8000", file=sys.stderr)
        raise SystemExit(2)
    aus = []
    for zeile in open(PFAD, encoding="utf-8"):
        zeile = zeile.strip()
        if not zeile:
            continue
        try:
            aus.append(json.loads(zeile))
        except json.JSONDecodeError:
            continue          # eine abgeschnittene letzte Zeile macht den Rest nicht wertlos
    return aus


# `signal_2` sagt, über welchen Bildschirm eine Antwort kam. Damit ist der Fluss auch ohne jede
# zusätzliche Meldung der Oberfläche lesbar — die Ankreuzliste zum Beispiel ist allein hieraus zu
# erkennen (nachgemessen 2026-08-27).
SCHIRME = {"klick": "Fragebogen", "hold": "Prüfliste (gehalten)", "rueckfrage": "Nachfrage",
           "verstanden": "Prüfliste", "screening": "Ankreuzliste", "konflikt": "Konflikt"}


def _uhr(e: dict) -> str:
    return (e.get("ts") or "")[11:19]


def zeige(eintraege: list[dict]) -> None:
    stand_offen = None
    letzter_kopf = None
    for e in eintraege:
        art, i, t = e.get("art"), e.get("inhalt") or {}, _uhr(e)

        if art == "fragen":
            kopf = [q["feld_id"] for q in i.get("kopf", [])]
            # Nur melden, wenn sich WAS ÄNDERT. /fragen wird nach jeder Antwort geholt; jeden
            # Aufruf zu zeigen hiesse, den Fluss unter Wiederholungen zu begraben.
            if kopf == letzter_kopf and i.get("offen") == stand_offen:
                continue
            letzter_kopf, stand_offen = kopf, i.get("offen")
            print(f"\n{t}  ── noch offen: {i.get('offen')} ── als Nächstes:")
            for q in i.get("kopf", []):
                inst = f"  [{q['instanzen']}x]" if (q.get("instanzen") or 1) > 1 else ""
                print(f"          {q['feld_id']:42s}{inst} {q.get('frage', '')[:74]}")

        elif art == "antwort":
            weg = (i.get("weg") or "").split("@")[0] or ""
            marke = SCHIRME.get(weg) or (
                "KI-Vorschlag" if str(i.get("schreiber", "")).startswith("llm")
                else f"automatisch ({i.get('schreiber')})" if not weg else weg)
            korr = "  (ersetzt)" if i.get("ersetzt") else ""
            print(f"{t}  → {marke:22s} {i.get('feld_id'):40s} = "
                  f"{str(i.get('wert'))[:34]}{korr}")

        elif art == "weg_gewaehlt":
            print(f"\n{t}  ══ Weg gewählt: {i.get('weg')!r}")

        elif art == "nachfragen_gestartet":
            print(f"\n{t}  ── Nachfragen: {len(i.get('gestellt') or [])} werden gestellt, "
                  f"{len(i.get('entfallen') or [])} entfielen vorher, "
                  f"{i.get('zurueckgestellt', 0)} in den Fragebogen verschoben")
            for rf in i.get("gestellt") or []:
                print(f"          {rf.get('feld_id'):38s} „{rf.get('frage', '')[:64]}“")
            for f in i.get("entfallen") or []:
                print(f"          entfallen (Feld nicht mehr offen): {f}")

        elif art == "nachfrage_spaeter":
            print(f"{t}  ↷ ÜBERSPRUNGEN        {str(i.get('feld_id')):40s} "
                  f"„{i.get('frage', '')[:44]}“")

        elif art == "pruefliste_aendern":
            print(f"{t}  ✎ „Ändern“ gedrückt   {str(i.get('feld_id')):40s} "
                  f"(vorgeschlagen war {i.get('war')!r})")

        elif art == "pruefliste_weiter":
            un = i.get("unbestaetigt") or []
            print(f"{t}  ── Prüfliste verlassen, {len(un)} Zeile(n) unbestätigt"
                  + (f": {', '.join(str(x) for x in un[:8])}" if un else ""))

        elif art == "nutzertext":
            print(f"\n{t}  ✎ Nutzer schreibt: „{(i.get('text') or '')[:88]}“")

        elif art == "abgewiesen":
            print(f"{t}  ✗ ABGEWIESEN         {i.get('feld_id'):42s} = "
                  f"{str(i.get('wert'))[:24]}  — {i.get('grund', '')[:70]}")

        elif art == "ki":
            print(f"\n{t}  ┌─ KI, Stufe {i.get('stufe')}: {i.get('was')}")
            for z in _ki_zeilen(i):
                print(f"{'':10}│  {z}")

        elif art == "ergebnis":
            n = i.get("offen_anzahl")
            zahl = i.get("zahl_cent")
            print(f"\n{t}  ══ ERGEBNIS: grund={i.get('grund')!r}, "
                  f"zahl={'—' if zahl is None else f'{zahl / 100:.2f} EUR'}, "
                  f"{n} offene Felder benannt")
            for f in i.get("offen", []):
                print(f"{'':10}   offen: {f}")
            if i.get("grund") not in (None, "bestaetigt") and not n:
                print(f"{'':10}   ⚠ Ein Grund OHNE benanntes Feld — genau das sieht der Nutzer "
                      f"als „noch offen“, ohne zu erfahren, woran es liegt.")


def _ki_zeilen(i: dict):
    inh = i.get("inhalt")
    if i.get("was") == "aussagen" and isinstance(inh, list):
        for a in inh:
            yield f"gelesen [{a.get('status')}]: {a.get('text')}   ← „{a.get('beleg')}“"
    elif i.get("was") == "zuordnungen" and isinstance(inh, dict):
        yield f"Themen: {', '.join(sorted(inh.get('regeln') or [])) or '—'}"
    elif i.get("was") == "ergebnis" and isinstance(inh, dict):
        for v in inh.get("vorschlaege") or []:
            yield f"Vorschlag: {v.get('feld_id')} = {v.get('wert')!r}   ← „{v.get('beleg')}“"
        for r in inh.get("rueckfragen") or []:
            yield f"Rückfrage: {r.get('feld_id')} — {r.get('frage')}"
        if inh.get("ohne_beleg_verworfen"):
            yield f"ohne Beleg verworfen: {inh['ohne_beleg_verworfen']}"
    else:
        yield json.dumps(inh, ensure_ascii=False)[:200]


def main(argv: list[str]) -> int:
    eintraege = lies()
    faelle = [f for f in dict.fromkeys(e.get("fall") for e in eintraege) if f]
    if "--faelle" in argv:
        for f in faelle:
            n = sum(1 for e in eintraege if e.get("fall") == f)
            print(f"{f}   {n} Einträge")
        return 0
    ziel = next((a for a in argv[1:] if not a.startswith("-")), faelle[-1] if faelle else None)
    if not ziel:
        print("Im Mitschnitt steht kein Fall.", file=sys.stderr)
        return 2
    dran = [e for e in eintraege if e.get("fall") == ziel]
    print(f"Fall {ziel} — {len(dran)} Einträge aus {PFAD}")
    zeige(dran)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
