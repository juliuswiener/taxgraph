"""Item-Registry je Regel - die monoton wachsende Menge aller je gefundenen
Pruef-Items, gegen die die Gates deterministisch pruefen.

Protokolldekret 2026-07-10, Stufe 4 (Registry-Ratsche). Die Nicht-Saettigung des
Inventars wird vom Problem zum Mechanismus: der Judge ist ein Detektor, jeder Lauf
fuettert die Registry, und NUR die Triage durch Julius aendert die Registry, NUR die
Registry aendert Verdikte. Damit gilt: identischer Registry-Stand -> identisches
Verdikt, per Konstruktion.

Ablauf (AINA):
  1. Der Judge findet Items (Detektor).
  2. `abgleich` matcht sie gegen die Registry ueber den Anker.
     - bekanntes Item -> traegt seinen Triage-Status bei,
     - neues Item     -> Discovery-Queue, kippt KEIN Gate.
  3. Julius triagiert neue Items -> erst das aendert die Registry.
  4. Die Gates pruefen gegen die Registry, nicht gegen den frischen Lauf.

Triage-Status eines registrierten Items:
  bedingung_neu        - abgedeckt durch eine Geltungsbedingung (`bedingung` gesetzt)
  grenzfall            - objektiv ambig -> routet in den Review (grenzfall-Gate)
  nicht_material       - generisch (Konvention) oder auf eine bestehende Bedingung
                         gemappt; blockiert nie
  nicht_echt           - kein echter Befund
  defekt_formalisierer - KEINE Annahme, sondern ein Formalisierungsfehler. Darf NIE
                         Bedingung werden (das legalisierte den Fehler). Marker,
                         gekoppelt an freigabe: blockiert; erlischt mit dem A-Neulauf.
  offen_bis_neuschnitt - Kern-Anwendbarkeit, gehoert beim Charge-Neuschnitt in
                         Signatur ODER Bedingung. Bis dahin registriert, blockiert
                         die (zuschnitt_offen-)Regel nicht.

Datei je Regel: pipeline/item_registry/<rule_id>.yaml
"""

from __future__ import annotations

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
REG_DIR = os.path.join(HERE, "item_registry")

sys.path.insert(0, ROOT)
sys.path.insert(0, HERE)
from yamlstrict import load_yaml   # noqa: E402
import gates as G                   # noqa: E402

TRIAGE = ("bedingung_neu", "grenzfall", "nicht_material", "nicht_echt",
          "defekt_formalisierer", "offen_bis_neuschnitt", "nicht_material_backlog")


def _anker(art: str, item: dict) -> list:
    """Stabiler Anker eines Verdikt-Items (wie in judge._anker, hier aus dem
    fertigen Verdikt rekonstruiert)."""
    if art == "norm_teil":
        import re
        ref = re.sub(r"\s+", " ", str(item.get("referenz") or "?")).strip().lower()
        return ["ref", ref or "?"]
    betrifft = G._normalize(str(item.get("betrifft") or "?")) or "?"
    if art == "abweichung":
        # Nicht-degeneriert (statt frueher immer ["betrifft","?"]): `betrifft`
        # wenn vorhanden (dict-Abweichung, im Judge-Schema verpflichtend), sonst
        # ein normalisierter Text-Anker aus der Aussage. Verschiedene Abweichungen
        # -> verschiedene Schluessel; dieselbe Abweichung ueber Laeufe -> derselbe.
        # Kein Sammel-Schluessel mehr, der jede kuenftige echte Abweichung als
        # "bekannt" schluckte (stilles Gruen).
        if betrifft != "?":
            return ["betrifft", betrifft]
        txt = G._normalize(str(item.get("aussage") or item.get("text") or ""))
        return ["abweichung_text", txt or "?"]
    return ["betrifft_kat", betrifft, str(item.get("kategorie") or "sonstige")]


def _key(art: str, anker: list) -> str:
    import json
    return json.dumps([art] + list(anker), ensure_ascii=False)


def load(rule_id: str) -> dict:
    p = os.path.join(REG_DIR, f"{rule_id}.yaml")
    if not os.path.exists(p):
        return {"rule_id": rule_id, "version": 0, "items": []}
    d = load_yaml(p) or {}
    d.setdefault("items", [])
    return d


def _index(reg: dict) -> dict[str, dict]:
    return {_key(it["art"], it["anker"]): it for it in reg.get("items", [])}


def abgleich(reg: dict, verdict: dict) -> dict:
    """Matcht ein frisches Verdikt gegen die Registry.

    Rueckgabe: {"bekannt": [reg-items mit Vorkommen], "neu": [Discovery-Items]}.
    Neue Items kippen kein Gate - sie warten auf Triage.
    """
    idx = _index(reg)
    bekannt, neu = [], []
    felder = [("abweichung", verdict.get("abweichungen", [])),
              ("annahme", verdict.get("stille_zusatzannahmen", [])),
              ("norm_teil", verdict.get("scope_gap", []))]
    for art, items in felder:
        for it in items:
            # Abweichungen sind hier Strings, Annahmen/Gaps sind Dicts
            item = {"aussage": it} if isinstance(it, str) else it
            k = _key(art, _anker(art, item))
            if k in idx:
                bekannt.append(idx[k])
            else:
                neu.append({"art": art, "schluessel": k, "anker": _anker(art, item),
                            "text": (item.get("norm_teil") or item.get("annahme")
                                     or item.get("aussage") or it)[:200],
                            "klasse": item.get("klasse"),
                            "kategorie": item.get("kategorie"),
                            "abgedeckt_von": item.get("abgedeckt_von"),
                            "bedingung_id": item.get("bedingung_id")})
    return {"bekannt": bekannt, "neu": neu}


# -- deterministische Gates gegen die Registry --------------------------------

def geltungsbereich_luecken(reg: dict, rule: dict) -> list[dict]:
    """Registrierte wirkt_hinein-Items mit Triage `bedingung_neu`, deren Bedingung
    NICHT im Manifest deklariert ist. Rein aus Registry + Manifest -> deterministisch."""
    deklariert = {b["bedingung"] for b in (rule.get("geltungsbedingungen") or [])}
    offen = []
    for it in reg.get("items", []):
        if it.get("art") != "norm_teil" or it.get("klasse") != "wirkt_hinein":
            continue
        if it.get("triage") == "bedingung_neu" and it.get("bedingung") not in deklariert:
            offen.append(it)
    return offen


def roundtrip_luecken(reg: dict, rule: dict) -> list[dict]:
    """Registrierte Annahmen mit Triage `bedingung_neu`, deren Bedingung nicht
    deklariert ist. `nicht_material`/`nicht_echt` blockieren nie."""
    deklariert = {b["bedingung"] for b in (rule.get("geltungsbedingungen") or [])}
    return [it for it in reg.get("items", [])
            if it.get("art") == "annahme" and it.get("triage") == "bedingung_neu"
            and it.get("bedingung") not in deklariert]


def grenzfaelle(reg: dict) -> list[dict]:
    return [it for it in reg.get("items", []) if it.get("triage") == "grenzfall"]


def defekte(reg: dict) -> list[dict]:
    return [it for it in reg.get("items", [])
            if it.get("triage") == "defekt_formalisierer"]


def registrierte_bedingungen(reg: dict) -> set[str]:
    return {it["bedingung"] for it in reg.get("items", [])
            if it.get("triage") == "bedingung_neu" and it.get("bedingung")}


def geltungsbereich_gate(reg: dict, rule: dict) -> "G.GateResult":
    """Deterministisch: jedes registrierte wirkt_hinein-Item ist abgedeckt.

    Haengt nur von Registry + Manifest ab, nicht vom frischen Lauf. Identischer
    Registry-Stand -> identisches Verdikt.
    """
    offen = geltungsbereich_luecken(reg, rule)
    if offen:
        return G.GateResult("geltungsbereich", G.FAIL,
                            f"{len(offen)} registrierte(s) wirkt_hinein-Item(s) ohne "
                            f"deklarierte Bedingung; erstes: "
                            f"{str(offen[0].get('anker'))[:80]}")
    n = sum(1 for it in reg.get("items", [])
            if it.get("art") == "norm_teil" and it.get("klasse") == "wirkt_hinein"
            and it.get("triage") == "bedingung_neu")
    return G.GateResult("geltungsbereich", G.PASS,
                        f"{n} registrierte(s) wirkt_hinein-Item(s) abgedeckt")


def roundtrip_gate(reg: dict, rule: dict) -> "G.GateResult":
    """Deterministisch: jede registrierte Annahme mit `bedingung_neu` ist deklariert.

    RESTRISIKO (Protokolldekret 2026-07-10, Punkt 5, woertlich): Nach der
    Vervollstaendigung der Bedingungslisten ist die verbleibende Aufgabe dieses
    Gates die Erkennung UNdeklarierter Annahmen, und dort bleibt Recall-Rauschen
    ein False-PASS-Risiko - eine Annahme, die alle Inventarlaeufe verpassen. Das
    Gate garantiert KEINE Vollstaendigkeit. Gegenlager: Union-until-Saturation,
    Golden-Tests, Human-Review.
    """
    offen = roundtrip_luecken(reg, rule)
    if offen:
        return G.GateResult("roundtrip", G.FAIL,
                            f"{len(offen)} registrierte Annahme(n) mit Bedingung, die "
                            f"nicht deklariert ist; erste: {str(offen[0].get('anker'))[:80]}")
    return G.GateResult("roundtrip", G.PASS,
                        "alle registrierten Annahmen deklariert oder als "
                        "nicht_material/nicht_echt triagiert")


def grenzfall_gate(reg: dict) -> "G.GateResult":
    gf = grenzfaelle(reg)
    if gf:
        return G.GateResult("grenzfall", G.FAIL,
                            f"{len(gf)} objektiv ambige(s) Item(s) -> Review")
    return G.GateResult("grenzfall", G.PASS, "keine Grenzfaelle in der Registry")


def defekt_gate(reg: dict) -> "G.GateResult":
    """Ein registrierter Formalisierungsfehler blockiert die Freigabe.

    `defekt_formalisierer` ist keine Annahme, sondern ein bekannter A-Fehler (z.B.
    Rundung ohne Normbefehl). Er darf nie eine Bedingung werden - das legalisierte
    den Fehler. Der Marker haelt die Regel im Review, bis der A-Neulauf ihn
    aufloest."""
    d = defekte(reg)
    if d:
        return G.GateResult("defekt", G.FAIL,
                            f"{len(d)} registrierte(r) Formalisierungsfehler -> "
                            f"freigabe blockiert bis A-Neulauf")
    return G.GateResult("defekt", G.PASS, "kein registrierter Formalisierungsfehler")


def discovery_gate(neu: list[dict]) -> "G.GateResult":
    """Informativ, nicht blockierend im Sinne der Gate-Ausgabe: neue Funde kippen
    NIE ein Gate (Punkt 3). Sie erzeugen Review-Items, die Julius triagiert. Der
    Queue-Status behandelt offene Discovery getrennt."""
    if neu:
        return G.GateResult("discovery", G.SKIP,
                            f"{len(neu)} neue(s) Item(s) -> Triage-Queue (kippt kein Gate)")
    return G.GateResult("discovery", G.PASS, "keine neuen Items")


def gates_fuer(rule_id: str, rule: dict, verdict: dict) -> tuple[dict, list]:
    """Die Registry-getriebenen Judge-Gates plus Discovery-Liste.

    Rueckgabe: ({gate_name: GateResult}, discoveries). Die Gates haengen NUR an der
    Registry und am Manifest - der frische `verdict` liefert nur die Discoveries
    (neue Funde), die niemals ein Gate kippen.
    """
    if not verdict or verdict.get("parse_error") or verdict.get("truncated"):
        det = ("judge answer truncated at max_tokens" if (verdict or {}).get("truncated")
               else "judge output not valid JSON")
        gates = {n: G.GateResult(n, G.FAIL, det)
                 for n in ("roundtrip", "geltungsbereich", "grenzfall", "defekt")}
        gates["discovery"] = G.GateResult("discovery", G.FAIL, det)
        return gates, []
    # Falschgruen-Sperre im Regate-/Ableitungs-Pfad: ein bewusst uebersprungenes Judge-
    # Verdikt (skip_judge) ist KEIN Verdikt. gates_fuer darf es nicht als vollwertig lesen -
    # sonst faellt der Abgleich gegen eine leere Registry auf null Items und alle Registry-
    # Gates werden faelschlich PASS -> verified_bedingt. Judge-abhaengige Gates auf SKIP mit
    # ehrlichem Detail; der Status bleibt strukturgeprueft_judge_offen bis zum Judge-Nachzug.
    if verdict.get("skipped"):
        det = "judge uebersprungen - Nachzug ausstehend"
        gates = {n: G.GateResult(n, G.SKIP, det)
                 for n in ("roundtrip", "geltungsbereich", "grenzfall", "defekt", "scope_gap")}
        gates["discovery"] = G.GateResult("discovery", G.SKIP, det)
        return gates, []
    reg = load(rule_id)
    ab = abgleich(reg, verdict)
    gates = {
        "geltungsbereich": geltungsbereich_gate(reg, rule),
        "roundtrip": roundtrip_gate(reg, rule),
        "grenzfall": grenzfall_gate(reg),
        "defekt": defekt_gate(reg),
        "discovery": discovery_gate(ab["neu"]),
        "scope_gap": G.scope_gap_gate(verdict),   # informativ, aus dem Lauf
    }
    return gates, ab["neu"]


def _atomic_write(path: str, content: str) -> None:
    """Schreibt via temp-Datei + os.replace: entweder der alte oder der neue
    Inhalt, nie ein halb geschriebener. Ein Abbruch mitten im Schreiben darf die
    monoton wachsende Registry nicht truncaten."""
    import tempfile
    d = os.path.dirname(path) or "."
    os.makedirs(d, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=d, prefix=".tmp_", suffix=".yaml")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            f.write(content)
        os.replace(tmp, path)
    except BaseException:
        try:
            os.unlink(tmp)
        except OSError:
            pass
        raise


def save(reg: dict) -> str:
    import yaml
    os.makedirs(REG_DIR, exist_ok=True)
    reg["version"] = int(reg.get("version", 0)) + 1
    p = os.path.join(REG_DIR, f"{reg['rule_id']}.yaml")
    kopf = ("# Item-Registry (Protokolldekret Stufe 4). Monoton wachsend.\n"
            "# Nur Julius' Triage aendert diese Datei; nur diese Datei aendert Verdikte.\n\n")
    _atomic_write(p, kopf + yaml.safe_dump(reg, allow_unicode=True, sort_keys=False))
    return p


# -- Triage-Werkzeug ----------------------------------------------------------

def discover_draft(rule_id: str) -> dict:
    """Erzeugt aus dem letzten Produktions-Verdikt einen Triage-Entwurf: alle
    noch nicht registrierten Items mit `triage: offen`. Julius setzt die Triage,
    dann nimmt `aufnehmen` sie in die Registry auf."""
    import json
    rep_p = os.path.join(ROOT, "pipeline", "runs", "produktion", rule_id, "report.json")
    if not os.path.exists(rep_p):
        raise SystemExit(f"kein Report fuer {rule_id}")
    v = json.load(open(rep_p, encoding="utf-8")).get("judge_verdict") or {}
    reg = load(rule_id)
    ab = abgleich(reg, v)
    items = []
    for n in ab["neu"]:
        e = {"art": n["art"], "anker": n["anker"], "text": n["text"], "bedingung": ""}
        if n["art"] == "norm_teil":
            # Daempfung 2a (Dekret 2026-07-11): Norm-Teile OHNE wirkt_hinein-Urteil
            # sind per Abgrenzungsregel + "scope_gap ist informativ" KEINE
            # Coverage-Items -> Formalisierungs-Backlog (backlog.py), kein Julius.
            klasse = n.get("klasse")
            if klasse:
                e["klasse"] = klasse
            deck = n.get("abgedeckt_von")
            if klasse == "unabhaengig":
                e["triage"] = "nicht_material_backlog"
            elif klasse == "wirkt_hinein" and isinstance(deck, str) and deck and deck != "none":
                # wirkt_hinein + Detektor mappt auf eine Bedingung -> Vorschlag
                e["triage"] = "bedingung_neu"
                e["bedingung"] = deck
                e["vorschlag"] = "detektor"
            else:
                # wirkt_hinein ohne Deckung, oder Klasse fehlt -> Julius entscheidet
                e["triage"] = "offen"
            items.append(e)
            continue
        # Annahmen / Abweichungen: Detektor-Vorschlag aus bedingung_id (AINA):
        #  konv:X            -> nicht_material (globale Konvention)
        #  deklarierte Bed.  -> bedingung_neu mit dieser Bedingung
        #  None              -> offen (Julius entscheidet)
        bid = n.get("bedingung_id")
        if isinstance(bid, str) and bid.startswith("konv:"):
            e["triage"] = "nicht_material"
            e["konvention"] = bid
        elif isinstance(bid, str) and bid:
            e["triage"] = "bedingung_neu"
            e["bedingung"] = bid
            e["vorschlag"] = "detektor"
        else:
            e["triage"] = "offen"
        if n.get("klasse"):
            e["klasse"] = n["klasse"]
        items.append(e)
    return {"rule_id": rule_id, "hinweis": "triage je Item setzen: bedingung_neu | "
            "grenzfall | nicht_material | nicht_echt; bei bedingung_neu die "
            "bedingung-ID eintragen. Dann: python pipeline/item_registry.py aufnehmen <datei>",
            "items": items}


def aufnehmen(draft: dict) -> tuple[str, int]:
    """Nimmt triagierte Items in die Registry auf (monoton). Items mit
    `triage: offen` werden uebersprungen. Rueckgabe: (pfad, aufgenommen)."""
    rid = draft["rule_id"]
    reg = load(rid)
    idx = _index(reg)
    n = 0
    for it in draft.get("items", []):
        tri = it.get("triage")
        if tri == "offen" or tri not in TRIAGE:
            continue
        k = _key(it["art"], it["anker"])
        e = idx.get(k) or {"art": it["art"], "anker": it["anker"],
                           "formulierungen": []}
        if it.get("text") and it["text"] not in e.get("formulierungen", []):
            e.setdefault("formulierungen", []).append(it["text"])
        e["triage"] = tri
        if it.get("klasse"):
            e["klasse"] = it["klasse"]
        if tri == "bedingung_neu":
            if not it.get("bedingung"):
                raise SystemExit(f"triage bedingung_neu ohne bedingung-ID: {k}")
            e["bedingung"] = it["bedingung"]
        if tri == "defekt_formalisierer" and it.get("bedingung"):
            # Ein Formalisierungsfehler darf nie eine Bedingung werden - das
            # legalisierte den Fehler.
            raise SystemExit(f"defekt_formalisierer darf keine bedingung tragen: {k}")
        if k not in idx:
            reg.setdefault("items", []).append(e)
            idx[k] = e
        n += 1
    return save(reg), n


def seed_mechanisch(rule_id: str, triage_eintraege: list[dict],
                    verdict: dict) -> tuple[int, list[dict]]:
    """Mechanisches Seeding (Protokolldekret 2026-07-11, Punkt 2).

    Ein frisches Item uebernimmt einen Triage-Status NUR, wenn sein Anker EXAKT
    einem Eintrag der Triage entspricht (fuer Annahmen: betrifft+kategorie; fuer
    Norm-Teile: referenz). KEINE Aehnlichkeits-Zuordnung. Items ohne exakte
    Entsprechung gehen in die Discovery-Queue und warten auf Julius.

    Rueckgabe: (aufgenommen, offene_discovery-items).
    """
    tri_idx = {_key(e["art"], e["anker"]): e for e in triage_eintraege}
    reg = load(rule_id)
    ab = abgleich(reg, verdict)
    aufnahme = []
    offen = []
    for n in ab["neu"]:
        e = tri_idx.get(n["schluessel"])
        if e is None:
            offen.append(n)                    # keine Entsprechung -> Discovery
            continue
        eintrag = {"art": n["art"], "anker": n["anker"],
                   "formulierungen": [n["text"]], "triage": e["triage"]}
        if n.get("klasse"):
            eintrag["klasse"] = n["klasse"]
        if e["triage"] == "bedingung_neu":
            eintrag["bedingung"] = e["bedingung"]
        aufnahme.append(eintrag)
    if aufnahme:
        idx = _index(reg)
        for e in aufnahme:
            k = _key(e["art"], e["anker"])
            if k not in idx:
                reg.setdefault("items", []).append(e)
                idx[k] = e
        save(reg)
    return len(aufnahme), offen


def main() -> int:
    if len(sys.argv) < 3:
        print("Verwendung:\n"
              "  python pipeline/item_registry.py discover <rule_id> [> draft.yaml]\n"
              "  python pipeline/item_registry.py aufnehmen <draft.yaml>")
        return 2
    import yaml
    cmd = sys.argv[1]
    if cmd == "discover":
        print(yaml.safe_dump(discover_draft(sys.argv[2]), allow_unicode=True,
                             sort_keys=False))
        return 0
    if cmd == "aufnehmen":
        draft = load_yaml(sys.argv[2])
        p, n = aufnehmen(draft)
        print(f"{n} Item(s) triagiert -> {p} (version {load(draft['rule_id'])['version']})")
        return 0
    print(f"unbekannt: {cmd}")
    return 2


if __name__ == "__main__":
    sys.exit(main())
