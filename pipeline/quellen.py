"""Multi-Source-Tasks: eine Regel wird aus mehreren etikettierten Quellen gebildet.

Eine Norm allein reicht oft nicht. § 33 Abs. 3 EStG laesst sich aus dem Wortlaut
in zwei Richtungen lesen; erst der BFH (VI R 75/14) legt fest, dass die
Prozentsaetze stufenweise anzuwenden sind. Genau diese Auslegung hat Formalisierer
A in Charge 1 verfehlt - nicht aus Unvermoegen, sondern weil sie im Input fehlte.

Der Ausweg ist nicht, den Normtext anzureichern (dann verschwimmt, was Gesetz ist
und was Auslegung), sondern mehrere Quellen getrennt und etikettiert zu uebergeben:

    quellen:
    - typ: gesetz
      label: "Norm: § 33 Abs. 3 EStG"
      datei: sources/gesetze-im-internet/estg_p33_abs3_2026-07-09.txt
    - typ: rechtsprechung
      label: "Bindende Auslegung: BFH v. 19.01.2017 - VI R 75/14, Leitsatz 1"
      datei: sources/bfh/bfh_vi_r_75-14_2017-01-19.txt
      ecli: "ECLI:DE:BFH:2017:U.190117.VIR75.14.0"
      zitatanker: "..."
      auszug: "..."      # woertlicher Ausschnitt, der ans Modell geht

Zwei harte Vorbedingungen, geprueft BEVOR ein Modell laeuft (ein Verstoss ist ein
Abbruch, keine Warnung):

  1. Jeder `zitatanker` muss - nach Normalisierung - im eingefrorenen Quelltext
     vorkommen.
  2. Jeder `auszug`, der ans Modell geht, muss woertlich aus der Datei stammen.
     Ohne diese Pruefung koennte ein Auszug still von der Quelle abweichen und das
     Modell auf einen Text ansetzen, den niemand eingefroren hat.

`typ` ist die Etikettierung, die im Prompt sichtbar bleibt: das Modell soll wissen,
was Gesetzestext ist und was ihn auslegt. Der Mechanismus ist generisch - jede
BMF-konkretisierte Regel braucht ihn.
"""

from __future__ import annotations

import os

from gates import _normalize

# Reihenfolge im Prompt: Gesetz zuerst, dann was es auslegt.
_RANG = {"gesetz": 0, "rechtsprechung": 1, "verwaltung": 2}

_TYP_HINWEIS = {
    "gesetz": "Gesetzestext. Massgeblich.",
    "rechtsprechung": ("Hoechstrichterliche Auslegung des obigen Gesetzestextes. "
                       "Sie ist bindend und geht einer abweichenden Lesart des "
                       "Wortlauts vor."),
    "verwaltung": ("Verwaltungsauffassung (BMF/EStH). Konkretisiert den "
                   "Gesetzestext, geht dem Gesetzeswortlaut aber NICHT vor."),
}


class QuellenFehler(RuntimeError):
    pass


def _read(root: str, rel: str) -> str:
    path = os.path.join(root, rel)
    if not os.path.exists(path):
        raise QuellenFehler(f"Quelle fehlt: {rel}")
    return open(path, encoding="utf-8").read()


def quellen_of(rule: dict) -> list[dict]:
    """Normalisiert das Manifest: `norm_source` (alt) oder `quellen` (neu)."""
    if rule.get("quellen"):
        return list(rule["quellen"])
    return [{"typ": "gesetz",
             "label": rule.get("norm") or "Norm",
             "datei": rule["norm_source"]}]


def build_norm_text(rule: dict, root: str) -> tuple[str, list[dict]]:
    """Return (prompt_text, quellen_meta). Raises QuellenFehler on any violation."""
    quellen = sorted(quellen_of(rule), key=lambda q: _RANG.get(q.get("typ"), 9))
    bloecke, meta = [], []
    for q in quellen:
        typ = q.get("typ")
        if typ not in _TYP_HINWEIS:
            raise QuellenFehler(f"unbekannter Quellen-Typ {typ!r} in {rule['rule_id']}")
        text = _read(root, q["datei"])
        norm_text = _normalize(text)

        anker = q.get("zitatanker")
        if anker and _normalize(anker) not in norm_text:
            raise QuellenFehler(
                f"{rule['rule_id']}: Zitatanker nicht im eingefrorenen Quelltext "
                f"({q['datei']}): {anker[:80]!r}")

        auszug = q.get("auszug")
        if auszug:
            if _normalize(auszug) not in norm_text:
                raise QuellenFehler(
                    f"{rule['rule_id']}: `auszug` steht nicht woertlich in "
                    f"{q['datei']}. Ein Auszug, der ans Modell geht, muss aus der "
                    f"eingefrorenen Quelle stammen.")
            inhalt = auszug.strip()
        else:
            inhalt = text.strip()

        kopf = f"=== [{typ}] {q.get('label', q['datei'])}"
        if q.get("ecli"):
            kopf += f" ({q['ecli']})"
        kopf += " ==="
        bloecke.append(f"{kopf}\n{_TYP_HINWEIS[typ]}\n\n{inhalt}")
        meta.append({k: q[k] for k in ("typ", "label", "datei", "ecli", "zitatanker")
                     if k in q})

    # D0: jedes deckt_ab-Fragment der Geltungsbedingungen muss woertlich im Freeze
    # stehen (Vorbedingung 3, analog zitatanker/auszug). Erste Verletzung = Abbruch.
    viol = deckt_ab_freeze_verletzungen(rule, root)
    if viol:
        rid, bid, frag, dateien = viol[0]
        raise QuellenFehler(
            f"{rid}: deckt_ab-Fragment der Geltungsbedingung {bid!r} steht nicht "
            f"woertlich in {dateien}: {frag[:80]!r}")

    return "\n\n".join(bloecke), meta


# -- deckt_ab-Freeze-Gate (D0) + Multi-Fragment/per-datei (D1) + Diskriminanz (D4) -


def _regel_dateien(rule: dict) -> list[str]:
    """Default-Quelldateien einer Regel (Union ihrer `quellen`), gegen die ein
    deckt_ab-Fragment geprueft wird, wenn die Bedingung keinen `datei`-Zeiger traegt.
    Dedupliziert (Reihenfolge erhalten): mehrere `quellen`-Eintraege koennen auf
    dieselbe Datei zeigen - sonst zaehlte die Diskriminanz denselben Treffer mehrfach."""
    return list(dict.fromkeys(q["datei"] for q in quellen_of(rule) if q.get("datei")))


def deckt_ab_fragmente(bedingung: dict) -> list[str]:
    """D1: `deckt_ab` ist String (1 Fragment) ODER Liste (Multi-Fragment). Eine
    Bedingung kann mehrere Norm-Teile ueber mehrere Absaetze abdecken."""
    d = bedingung.get("deckt_ab", "")
    frags = d if isinstance(d, list) else [d]
    return [f for f in frags if f]


def _bedingung_dateien(bedingung: dict, regel_dateien: list[str]) -> list[str]:
    """D1: expliziter `datei`-Zeiger der Bedingung (Cross-Source-Override) ODER
    Default = Quelldateien der Regel."""
    datei = bedingung.get("datei")
    return [datei] if datei else regel_dateien


def _norm_datei_cache(root: str, cache: dict, rel: str) -> str | None:
    if rel not in cache:
        p = os.path.join(root, rel)
        cache[rel] = _normalize(open(p, encoding="utf-8").read()) if os.path.exists(p) else None
    return cache[rel]


def deckt_ab_freeze_verletzungen(rule: dict, root: str, cache: dict | None = None
                                 ) -> list[tuple]:
    """D0: liefert (rule_id, bedingung, fragment, dateien) je deckt_ab-Fragment, das
    - nach _normalize - in KEINER seiner aufgeloesten Freeze-Dateien vorkommt. Leer =
    alle Anker freeze-gedeckt."""
    cache = {} if cache is None else cache
    regel_dateien = _regel_dateien(rule)
    out = []
    for b in rule.get("geltungsbedingungen") or []:
        dateien = _bedingung_dateien(b, regel_dateien)
        for frag in deckt_ab_fragmente(b):
            nf = _normalize(frag)
            if not any((t := _norm_datei_cache(root, cache, dt)) and nf in t
                       for dt in dateien):
                out.append((rule["rule_id"], b.get("bedingung"), frag, dateien))
    return out


def deckt_ab_diskriminanz(rule: dict, root: str, min_len: int = 25,
                          cache: dict | None = None) -> list[tuple]:
    """D4 (INFO, nicht blockierend): kurze UND nicht-eindeutige deckt_ab-Fragmente.
    Geflaggt wird nur, wenn ein Fragment kuerzer als `min_len` Zeichen ist UND seine
    Treffer-Zahl in den aufgeloesten Freezes != 1 ist (reine Laenge wuerde kurze,
    aber eindeutige Anker unnoetig flaggen). Ein `schwach_ok: true` an der Bedingung
    ist ein bewusster Waiver und unterdrueckt die Meldung."""
    cache = {} if cache is None else cache
    regel_dateien = _regel_dateien(rule)
    out = []
    for b in rule.get("geltungsbedingungen") or []:
        if b.get("schwach_ok"):
            continue
        dateien = _bedingung_dateien(b, regel_dateien)
        for frag in deckt_ab_fragmente(b):
            if len(frag) >= min_len:
                continue
            nf = _normalize(frag)
            treffer = sum((t := _norm_datei_cache(root, cache, dt)) and t.count(nf) or 0
                          for dt in dateien)
            if treffer != 1:
                out.append((rule["rule_id"], b.get("bedingung"), frag, len(frag), treffer))
    return out
