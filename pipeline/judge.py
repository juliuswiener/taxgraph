"""Dekomponierter Judge mit Mehrheitsentscheid.

Der monolithische Judge (roundtrip_diff@4) lieferte bei identischem Input
verschiedene Verdikte: drei von sechs gemessenen Regeln bekamen wechselnde
Gate-Urteile, elf Prozent der Laeufe gar kein lesbares JSON, und § 35a brauchte
14.584 Tokens Freitext. Ein 14k-Token-Verdikt ist nicht deterministisch zu
bekommen, und sein Budget waechst mit der Prosa statt mit der Sache.

Deshalb zerfaellt der Judge in zwei Stufen:

  1. INVENTAR (`inventar@1`): listet nur die Pruef-Items - Abweichungskandidaten,
     Zusatzannahmen, Norm-Teile ausserhalb der Signatur. Je ein Satz, kein Urteil.
     Dreimal gefahren; ein Item zaehlt, wenn es in der MEHRHEIT der Inventare
     vorkommt (Abgleich ueber normalisierte Wortmengen). Was nur einmal auftaucht,
     ist Rauschen und wird als `inventar_streuung` protokolliert, nicht
     verschwiegen.

  2. URTEIL JE ITEM (`item_*@1`): ein Call pro Item, Mini-Schema, drei Stimmen,
     Mehrheit entscheidet. Ein 2:1-Split ist Information und wird am Report
     vermerkt (`judge_instability`).

Regeln aus dem Protokolldekret 2026-07-10:

  * Ein Parse-Fehler oder eine abgeschnittene Antwort ist KEINE Stimme. Es wird
    nachgelaufen, bis drei gueltige Verdikte vorliegen (begrenzt durch
    `max_versuche`).
  * Kommen keine drei gueltigen Stimmen zusammen, gilt das Item konservativ:
    eine Annahme ist `undeclared`, ein Norm-Teil `wirkt_hinein`, ein
    Abweichungskandidat `echt`. Schweigen darf nichts durchwinken.
  * Ein 2:1-Split auf einem blockierenden Gate ist kein stilles PASS oder FAIL,
    sondern eskaliert. Der Split ist der Befund.

Jedes Verdikt traegt `lauf_id` und `timestamp` des Judge-Laufs, der es erzeugt hat.
Ein Gate ohne frisches Verdikt hat keinen Zustand - niemals den alten.
"""

from __future__ import annotations

import hashlib
import json
import re
from collections import Counter
from concurrent.futures import ThreadPoolExecutor

from client import OpenRouterClient, RoleConfig
from prompts import build_messages
from provenance import stamp, Provenance, now_iso
from roles import signature_text, _bedingungen_block
import gates as G

STIMMEN = 3
MAX_VERSUCHE = 6          # pro Item bzw. pro Inventarlauf
AEHNLICHKEIT = 0.6        # Jaccard-Schwelle fuer "dasselbe Item"
PARALLEL = 3              # Stimmen eines Items sind unabhaengig und laufen parallel


# -- Hilfen -------------------------------------------------------------------

# Floskeln, die in fast jedem Item stehen und die Aehnlichkeit dominieren. Ohne
# sie verglich "Die Formalisierung nimmt an, dass anzahl_kinder ..." mit
# "Die Formalisierung nimmt an, dass monate_ohne_voraussetzung ..." zu 64 Prozent -
# zwei voellig verschiedene Annahmen. Uebrig bleiben die tragenden Begriffe:
# Eingabenamen, Paragraphen, Zahlen.
_FLOSKELN = {
    "die", "der", "das", "den", "dem", "des", "ein", "eine", "einer", "eines",
    "und", "oder", "dass", "wird", "werden", "wurde", "ist", "sind", "als",
    "nicht", "nur", "auch", "fuer", "von", "vom", "mit", "bei", "aus", "auf",
    "sich", "seiner", "seine", "ihre", "ihrer", "korrekte", "korrekten",
    "formalisierung", "nimmt", "setzt", "voraus", "annahme", "annimmt",
    "eingabe", "eingaben", "gelesen", "interpretiert", "verstanden", "behandelt",
    "zutreffende", "zutreffend", "anwendung", "beurteilung", "geht", "davon",
    "scope", "regel", "norm", "estg", "absatz", "abs", "satz", "nummer", "nr",
}


def _worte(text: str) -> set[str]:
    """Tragende Begriffe eines Items: Floskeln raus, kurze Woerter raus."""
    return {w for w in re.findall(r"\w+", G._normalize(text))
            if len(w) > 2 and w not in _FLOSKELN}


def _gleich(a: str, b: str) -> bool:
    """Zwei Item-Texte meinen dasselbe.

    Gemessen wird die Ueberdeckung der kleineren Wortmenge, nicht Jaccard: derselbe
    Befund wird mal knapp und mal ausfuehrlich formuliert ("Die Eingabe X ist ein
    Nettobetrag" vs. "Die Formalisierung nimmt an, dass die Eingabe X als
    Nettobetrag zu lesen ist"). Jaccard bestraft die Laengendifferenz und haette
    beide als verschiedene Items gezaehlt.
    """
    wa, wb = _worte(a), _worte(b)
    if not wa or not wb:
        return G._normalize(a) == G._normalize(b)
    return len(wa & wb) / min(len(wa), len(wb)) >= AEHNLICHKEIT


def _json_of(text: str | None) -> dict | None:
    if not text:
        return None
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        m = re.search(r"\{.*\}", text, re.DOTALL)
        if not m:
            return None
        try:
            return json.loads(m.group(0))
        except json.JSONDecodeError:
            return None


def _call(client: OpenRouterClient, role: RoleConfig, template: str, task: str,
          fixture: str, models_hash: str):
    import dataclasses
    r = dataclasses.replace(role, prompt_template_id=template, fewshot_set_id="")
    messages = build_messages(template, "", {"task_content": task})
    comp = client.complete(r, messages, fixture_id=fixture)
    return comp, stamp(r, comp, models_hash)


def _stimmen(client, role, template, task, fixture, models_hash, pruefe,
             provenance: list) -> tuple[list, int]:
    """Sammle STIMMEN gueltige Stimmen. Parse-Fehler/Truncation zaehlen nicht mit.

    Rueckgabe: (stimmen, ungueltige_versuche)
    """
    stimmen, ungueltig, versuche = [], 0, 0

    def einmal():
        comp, prov = _call(client, role, template, task, fixture, models_hash)
        d = None if comp.truncated else _json_of(comp.text)
        return (pruefe(d) if d is not None else None), prov

    # Die Stimmen eines Items sind unabhaengig - sie laufen parallel. Nachlaeufe
    # fuer ungueltige Antworten folgen sequenziell, weil ihre Zahl vom Ergebnis
    # abhaengt.
    with ThreadPoolExecutor(max_workers=PARALLEL) as pool:
        for wert, prov in pool.map(lambda _: einmal(), range(STIMMEN)):
            provenance.append(prov.to_dict())
            versuche += 1
            if wert is None:
                ungueltig += 1
            else:
                stimmen.append(wert)

    while len(stimmen) < STIMMEN and versuche < MAX_VERSUCHE:
        wert, prov = einmal()
        provenance.append(prov.to_dict())
        versuche += 1
        if wert is None:
            ungueltig += 1
        else:
            stimmen.append(wert)
    return stimmen, ungueltig


def _mehrheit(stimmen: list) -> tuple[object | None, bool]:
    """(gewinner, war_split). Ohne Mehrheit -> (None, True)."""
    if not stimmen:
        return None, True
    z = Counter(map(_key, stimmen))
    (top, n), = z.most_common(1)
    gewinner = next(s for s in stimmen if _key(s) == top)
    return (gewinner, n < len(stimmen))


def _key(wert) -> str:
    return json.dumps(wert, sort_keys=True, ensure_ascii=False)


# -- Stufe 1: Inventar --------------------------------------------------------

def _inventar(client, role, kontext: str, models_hash: str, provenance: list):
    felder = ("abweichungen", "annahmen", "norm_teile")

    def pruefe(d):
        if not isinstance(d, dict) or any(f not in d for f in felder):
            return None
        if any(not isinstance(d[f], list) for f in felder):
            return None
        return {f: [str(x)[:300] for x in d[f] if str(x).strip()] for f in felder}

    laeufe, ungueltig = _stimmen(client, role, "inventar@1", kontext, "judge_inventar",
                                 models_hash, pruefe, provenance)
    if not laeufe:
        return None, {"inventar_ungueltig": ungueltig}

    # VEREINIGUNG, nicht Mehrheit. Ein Item, das nur ein Inventarlauf sieht,
    # wird trotzdem beurteilt: es wegzulassen waere stilles Gruen, und genau das
    # ist die Fehlerklasse, die dieses Protokoll ausschliessen soll. Die
    # Item-Abstimmung filtert Rauschen ohnehin - ein erfundener Befund wird
    # dreimal als "nicht echt" beurteilt.
    #
    # Wie oft ein Item gesehen wurde, steht als `inventar_streuung` im Report:
    # das ist das Mass der Inventar-Instabilitaet, das Julius sehen will.
    ergebnis, streuung, merge_log = {}, {}, {}
    for f in felder:
        kandidaten: list[tuple[str, int]] = []
        merges: list[dict] = []
        for lauf in laeufe:
            gesehen = set()
            for text in lauf[f]:
                for i, (bekannt, _) in enumerate(kandidaten):
                    if _gleich(text, bekannt) and i not in gesehen:
                        kandidaten[i] = (bekannt, kandidaten[i][1] + 1)
                        gesehen.add(i)
                        if G._normalize(text) != G._normalize(bekannt):
                            # Zwei verschieden formulierte Rohtexte gelten als
                            # dasselbe Item. Das ist eine Entscheidung des
                            # Abgleichs, keine des Modells - sie gehoert
                            # nachpruefbar in den Report.
                            merges.append({"cluster": bekannt[:160],
                                           "eingeschmolzen": text[:160]})
                        break
                else:
                    kandidaten.append((text, 1))
                    gesehen.add(len(kandidaten) - 1)
        ergebnis[f] = [t for t, _ in kandidaten]
        streuung[f] = [{"text": t, "in_laeufen": n} for t, n in kandidaten
                       if n < len(laeufe)]
        merge_log[f] = merges
    return ergebnis, {"inventar_laeufe": len(laeufe), "inventar_ungueltig": ungueltig,
                      "inventar_streuung": streuung, "merge_log": merge_log,
                      "roh_items": {f: sum(len(l[f]) for l in laeufe) for f in felder},
                      "cluster": {f: len(ergebnis[f]) for f in felder}}


# -- Stufe 2: Urteil je Item --------------------------------------------------

def _urteil_annahme(client, role, kontext, bed_block, annahme, models_hash, prov, ids):
    def pruefe(d):
        m = d.get("mapping") if isinstance(d, dict) else None
        if not isinstance(m, str) or not m.strip():
            return None
        return {"mapping": m if m in ids else "undeclared"}

    task = f"{kontext}{bed_block}\n\nZu beurteilende Zusatzannahme:\n{annahme}"
    stimmen, ungueltig = _stimmen(client, role, "item_annahme@1", task,
                                  "judge_item", models_hash, pruefe, prov)
    gewinner, split = _mehrheit(stimmen)
    if gewinner is None:                       # keine gueltige Stimme -> konservativ
        return {"mapping": "undeclared"}, True, ungueltig, len(stimmen)
    return gewinner, split, ungueltig, len(stimmen)


def _urteil_normteil(client, role, kontext, bed_block, teil, models_hash, prov, ids):
    def pruefe(d):
        if not isinstance(d, dict):
            return None
        k = d.get("klasse")
        a = d.get("abgedeckt_von")
        if k not in ("wirkt_hinein", "unabhaengig"):
            return None
        return {"klasse": k, "abgedeckt_von": a if a in ids else "none"}

    task = f"{kontext}{bed_block}\n\nZu beurteilender Norm-Teil:\n{teil}"
    stimmen, ungueltig = _stimmen(client, role, "item_normteil@1", task,
                                  "judge_item", models_hash, pruefe, prov)
    gewinner, split = _mehrheit(stimmen)
    if gewinner is None:                       # konservativ: wirkt hinein
        return {"klasse": "wirkt_hinein", "abgedeckt_von": "none"}, True, ungueltig, 0
    return gewinner, split, ungueltig, len(stimmen)


def _urteil_abweichung(client, role, kontext, befund, models_hash, prov):
    def pruefe(d):
        v = d.get("ist_echt") if isinstance(d, dict) else None
        return {"ist_echt": bool(v)} if isinstance(v, bool) else None

    task = f"{kontext}\n\nZu beurteilender Abweichungskandidat:\n{befund}"
    stimmen, ungueltig = _stimmen(client, role, "item_abweichung@1", task,
                                  "judge_item", models_hash, pruefe, prov)
    gewinner, split = _mehrheit(stimmen)
    if gewinner is None:                       # konservativ: echt
        return {"ist_echt": True}, True, ungueltig, 0
    return gewinner, split, ungueltig, len(stimmen)


# -- oeffentliche API ---------------------------------------------------------

def judge_regel(client: OpenRouterClient, role: RoleConfig, norm_text: str,
                catala_src: str, signature: dict, geltungsbedingungen: list[dict],
                models_hash: str) -> tuple[dict, list[dict], float]:
    """Vollstaendiges Verdikt einer Regel. Rueckgabe: (verdict, provenance, kosten)."""
    provenance: list[dict] = []
    bedingungen = geltungsbedingungen or []
    ids = {b["bedingung"] for b in bedingungen}
    bed_block = _bedingungen_block(bedingungen)
    sig = (f"\n\nVorgegebene Scope-Signatur (die Grenze deiner Bewertung):\n"
           f"{signature_text(signature)}" if signature else "")
    kontext = (f"Original-Norm:\n{norm_text}{sig}\n\n"
               f"Catala-Formalisierung:\n{catala_src}")

    inventar, inv_meta = _inventar(client, role, kontext, models_hash, provenance)
    lauf_id = hashlib.sha256(
        (catala_src + now_iso() + str(len(provenance))).encode()).hexdigest()[:12]

    if inventar is None:
        verdict = {"parse_error": True, "lauf_id": lauf_id, "timestamp": now_iso(),
                   "judge_instability": inv_meta}
        return verdict, provenance, _kosten(provenance)

    instab = dict(inv_meta)
    instab["item_splits"] = []
    instab["items_ohne_mehrheit"] = []

    abweichungen = []
    for befund in inventar["abweichungen"]:
        urteil, split, ungueltig, n = _urteil_abweichung(client, role, kontext, befund,
                                                         models_hash, provenance)
        _vermerke(instab, "abweichung", befund, split, ungueltig, n)
        if urteil["ist_echt"]:
            abweichungen.append(befund)

    annahmen = []
    for annahme in inventar["annahmen"]:
        urteil, split, ungueltig, n = _urteil_annahme(client, role, kontext, bed_block,
                                                      annahme, models_hash, provenance, ids)
        _vermerke(instab, "annahme", annahme, split, ungueltig, n)
        bid = urteil["mapping"]
        annahmen.append({"annahme": annahme,
                         "bedingung_id": None if bid == "undeclared" else bid})

    gaps = []
    for teil in inventar["norm_teile"]:
        urteil, split, ungueltig, n = _urteil_normteil(client, role, kontext, bed_block,
                                                       teil, models_hash, provenance, ids)
        _vermerke(instab, "norm_teil", teil, split, ungueltig, n)
        gaps.append({"norm_teil": teil, "klasse": urteil["klasse"],
                     "begruendung": "",
                     "abgedeckt_von": urteil["abgedeckt_von"]})

    verdict = {
        "parse_error": False,
        "faithful": not abweichungen and all(a["bedingung_id"] for a in annahmen),
        "abweichungen": abweichungen,
        "stille_zusatzannahmen": annahmen,
        "scope_gap": gaps,
        "judge_instability": instab,
        "judge_protokoll": "dekomponiert@1 (Mehrheit aus 3 Stimmen je Item)",
        "lauf_id": lauf_id,
        "timestamp": now_iso(),
    }
    return verdict, provenance, _kosten(provenance)


def _vermerke(instab: dict, art: str, text: str, split: bool, ungueltig: int, n: int):
    if split:
        instab["item_splits"].append({"art": art, "item": text[:120],
                                      "gueltige_stimmen": n})
    if n < STIMMEN:
        instab["items_ohne_mehrheit"].append({"art": art, "item": text[:120],
                                              "gueltige_stimmen": n,
                                              "ungueltige_versuche": ungueltig})


def _kosten(provenance: list[dict]) -> float:
    return round(sum(p["cost_usd"] for p in provenance), 6)


def hat_split_auf_blockierendem_gate(verdict: dict) -> bool:
    """Ein 2:1-Split auf einem blockierenden Gate ist kein stilles PASS/FAIL.

    Blockierend sind `roundtrip` (Abweichungen, undeklarierte Annahmen) und
    `geltungsbereich` (wirkt_hinein ohne Abdeckung). Ein Split auf einem Item, das
    in eines dieser Gates eingeht, eskaliert an Julius.
    """
    for s in (verdict.get("judge_instability") or {}).get("item_splits", []):
        if s["art"] in ("abweichung", "annahme", "norm_teil"):
            return True
    return False
