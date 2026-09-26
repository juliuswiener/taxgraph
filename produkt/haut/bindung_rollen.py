"""Die Rollen der `bindung` an der Spannen-/Estimate-Naht — auseinandergehalten an EINER Stelle.

Aus `api.py` herausgezogen (2026-09-26): die Endpunkt-Schicht soll die Rollen nicht selbst
zusammensetzen, sie soll sie anfordern. Die Zeilenratsche `test_api_zeilen_ratsche` hat den
Umzug erzwungen, und ihr Einwand war sachlich richtig — Rechen-Naht-Logik gehört nicht in die
Endpunkt-Schicht.

DIE NAHT HAT DREI ROLLEN, NICHT ZWEI. Bis 2026-09-26 lagen zwei davon auf einer Variable
(`rb`), und daraus kam der Widerspruch Kopfzeile ↔ /ergebnis (240 EUR, Fall § 35a):

  1. ENUMERIEREN — `EM.instanzen(store, bindung, gruppe)` liest `b.get("instanz_gruppe")`,
     um zu wissen, welche Felder eine Instanz-Gruppe ausmachen. Braucht die VOLLE Bindung:
     mit dem Kegel sieht der § 35a-Topf nur dessen Felder, findet keine Instanz und summiert
     0 — er rechnet, als hätte der Nutzer nichts angegeben. (Der Kegel von `gesamt` enthält
     kein einziges Feld einer Instanz-Gruppe, gemessen.)

  2. ACHSEN — `intervall.py` bildet `askable` aus `b.get("askable")` und daraus `base`, die
     Wertemenge, über die die Spanne läuft. Braucht den KEGEL: mit der vollen Bindung werden
     341 statt 21 Felder zu Achsen, ein ungesetztes Partner-/nie gefragtes Feld zieht die
     Spanne auf `nicht_fixierbar` und die Kopfzeile zeigt „Noch keine Zahl" (gemessen: 251
     Felder; der naive Fix fällt damit durch).

  3. SLOT-ÜBERSETZUNG — `bescheid_via_slots` schließt die Aufbau-Bindung ein und liest
     `bindung[fid]["quelle"]["signatur_slot"]` beim Aufruf, je Feld aus `feld_werte`. Braucht
     eine OBERMENGE der Achsen-Bindung, und das ist die volle: `feld_werte` kommt aus `base`,
     `base` aus den Achsen — also Kegel ⊆ voll. Konstruktiv, nicht zufällig.

Die Reihenfolge im Rückgabewert von `rollen()` IST die Zusicherung: wer die zwei wieder zu
einer Variable zusammenzieht, verliert entweder die Instanzen (Rolle 1) oder die Zahl (Rolle 2).

Dass die volle Bindung auf der Aufbauseite keine Zahl VERSCHIEBEN kann, ist nicht nur diesmal
gemessen: `bindung[fid]` als *Wert* kommt in den Bescheid-Modulen NULL Mal vor (grep über
bescheid_*.py). Dort ist die Bindung ausschließlich Schlüsselmenge und Enumerations-Selektor,
nie ein Wertefilter — sie sieht mehr Felder, sie glaubt nicht mehr.
"""
from __future__ import annotations

import traverser as TR


def relevante_kegel_felder(scheibe_felder: tuple, bindung: dict, store: dict | None) -> tuple:
    """Der Pflicht-Kegel ohne die Felder, deren Regel der Nutzer selbst abbestellt hat.

    _feste_zahl verlangt für den Meet jedes Kegel-Feld als bestätigt. Ein Feld, dessen Regel
    traverser.relevanz() bereits ausgeschlossen hat, wird vom Traverser nie gefragt — es sperrte
    den Ring dauerhaft auf input_kegel_nicht_bestaetigt, ohne dass der Nutzer die Frage je zu
    sehen bekam (BACKLOG traverser-ring-kegel-relevanz-naht; gefunden am Fall vv_wohnzwecke=False,
    das p21_2_verbilligte_vermietung_wk ausschließt und vv_entgelt_quote_prozent im Kegel zurückließ).

    Fail-closed bleibt erhalten, und zwar an zwei Stellen:
    - relevanz() schließt NUR bei einem BESTÄTIGTEN False aus (traverser.py:122). Unbeantwortet
      oder vorläufig schließt NICHT aus, der Kegel bleibt also gesperrt, solange der Nutzer nicht
      geantwortet hat.
    - Ohne store (Alt-Aufrufer, Teil-Ringe) wird gar nichts ausgeschlossen — dann gilt der volle
      Kegel wie bisher.

    Nebenwirkung auf die slot_fn: ein weggelassenes Feld fehlt auch im Dict, das
    bescheid_via_slots an die slot_fn übergibt. Das ist geprüft und abgesichert — kein
    ausschließbares Kegel-Feld trägt einen Slot, den eine slot_fn liest
    (test_kegel_relevanz_naht::test_kein_ausschliessbares_kegelfeld_ist_ein_gelesener_slot)."""
    if store is None:
        return scheibe_felder
    rel = TR.relevanz(store, bindung)
    return tuple(f for f in scheibe_felder
                 if rel.get((bindung.get(f) or {}).get("quelle", {}).get("regel_id"),
                            {}).get("status") != "ausgeschlossen")


def ring_bindung(cfg: dict, bindung: dict, store: dict | None = None) -> dict:
    """Bindung für die Spannen-/intervall-Rechnung: nur die Pflicht-Kegel-Felder; mit `store` (nur für IV.intervall) ohne abbestellte.
    Sonst zögen ungesetzte Partner-Felder (einzel) und nie gefragte Felder als unbounded-ohne-Wert das Intervall auf nicht_fixierbar."""
    kegel = cfg.get("kegel")
    return {f: bindung[f] for f in relevante_kegel_felder(kegel, bindung, store) if f in bindung} if kegel else bindung


def rollen(cfg: dict, bindung: dict, store: dict | None = None) -> tuple[dict, dict]:
    """Die ZWEI Bindungen des Estimate-Pfads, einmal benannt: `(aufbau, achsen)`.

    Die drei Rollen und warum die Aufbau-Bindung keine Zahl verschieben kann, stehen im
    Modul-Docstring. Kurz: `aufbau` ist die VOLLE Bindung (Rolle 1: Enumerieren, Rolle 3:
    Slot-Übersetzung), `achsen` ist der KEGEL (Rolle 2: die Spannen-Achsen)."""
    return bindung, ring_bindung(cfg, bindung, store)
