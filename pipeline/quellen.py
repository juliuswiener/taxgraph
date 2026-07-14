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

    return "\n\n".join(bloecke), meta
