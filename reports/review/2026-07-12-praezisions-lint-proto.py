"""Prototyp Praezisions-Lint (Klasse 5) — NUR Design-Validierung, NICHT gates.py.

Erkennungsregel v1:
  (A) finaler Cent-Schnitt vorhanden:
      - Catala-Idiom  `truncate of (<expr> / $0.01)) * $0.01`  (auch floor/round), ODER
      - rundung-Deklaration mit richtung=floor UND deckt_ab nennt 'Cent'/'Bruchteile'.
  (B) money x decimal-Literal (Prozent-auf-Geld), das MONEY produziert und NICHT der
      Cent-Schnitt selbst ist: ein money-Name oder $-Literal, multipliziert/geteilt mit
      einem decimal-Literal (\\d+\\.\\d+, NICHT $-praefixiert).
  FLAG (Klasse-5-Verdacht) IFF (A) UND (B).

money-Namen = `input/output X content money` + `let Y equals <money-Ausdruck>` (Y erbt money,
wenn die RHS einen bekannten money-Namen oder ein $-Literal traegt und kein `/ $`-Cent-Quotient
oder `decimal`/`truncate`-Wrapper ist).
"""
import json, glob, os, re, yaml

CENT_CUT_IDIOM = re.compile(r"(truncate|round|floor|ceil(?:ing)?)\s+of\s*\(.*?/\s*\$0\.01", re.I)
DEC_LIT = r"(?<![\$\d])\d+\.\d+"                      # 0.055, 0.119 — NICHT $-praefixiert
MONEY_DECL = re.compile(r"\b(?:input|output|internal)\s+(\w+)\s+content\s+money\b")
LET_BIND = re.compile(r"\blet\s+(\w+)\s+equals\b(.*)$", re.M)


def money_names(src):
    names = set(MONEY_DECL.findall(src))
    # iterative Fixpunkt: let Y equals RHS -> Y money, wenn RHS money-Name/$-Literal traegt
    for _ in range(6):
        added = False
        for m in LET_BIND.finditer(src):
            y, rhs = m.group(1), m.group(2)
            if y in names:
                continue
            has_money = "$" in rhs or any(re.search(rf"\b{re.escape(n)}\b", rhs) for n in names)
            # v2 Typ-Tracking: money/money -> decimal. Y ist NICHT money, wenn die RHS
            # durch ein money-Literal ODER einen money-Namen TEILT (Quotient=decimal),
            # oder explizit `decimal of` traegt. Sonst erbt Y money vom money-Operanden.
            teilt_durch_money = bool(re.search(r"/\s*\$", rhs)) or \
                any(re.search(rf"/\s*{re.escape(n)}\b", rhs) for n in names)
            ist_decimal = teilt_durch_money or "decimal of" in rhs.lower()
            if has_money and not ist_decimal:
                names.add(y); added = True
        if not added:
            break
    return names


def has_final_cut(src, rundung):
    if CENT_CUT_IDIOM.search(src):
        return "catala-cent-idiom"
    for d in (rundung or []):
        if isinstance(d, dict) and str(d.get("richtung", "")).lower() == "floor":
            deckt = (d.get("deckt_ab", "") + d.get("zitatanker", "")).lower()
            if "cent" in deckt or "bruchteil" in deckt:
                return f"rundung floor '{d.get('zitatanker','')[:30]}'"
    return None


def money_times_decimal(src, mnames):
    """(B): Zeilen mit money-Operand [*/] decimal-Literal, ohne den Cent-Schnitt selbst."""
    hits = []
    for i, line in enumerate(src.splitlines(), 1):
        code = line.split("#", 1)[0]
        if re.search(r"/\s*\$0\.01", code):        # Cent-Schnitt-Quotient: kein (B)
            continue
        for m in re.finditer(rf"([\w.$]+)\s*([*/])\s*({DEC_LIT})", code):
            left = m.group(1)
            left_money = left.startswith("$") or left.strip("$") in mnames \
                or any(left.endswith(n) for n in mnames)
            if left_money:
                hits.append((i, code.strip(), m.group(0)))
        # decimal-Literal LINKS, money RECHTS: 0.119 * unterschiedsbetrag
        for m in re.finditer(rf"({DEC_LIT})\s*([*])\s*([\w$]+)", code):
            right = m.group(3)
            if right.startswith("$") or right in mnames:
                hits.append((i, code.strip(), m.group(0)))
    return hits


def lint(rid, src, rundung):
    cut = has_final_cut(src, rundung)
    mnames = money_names(src)
    bhits = money_times_decimal(src, mnames) if cut else []
    flag = bool(cut and bhits)
    return {"rid": rid, "flag": flag, "cut": cut, "money_x_decimal": bhits,
            "money_names": sorted(mnames)}


def main():
    cfg = yaml.safe_load(open("pipeline/produktion/rules.yaml"))
    rundung_by = {r["rule_id"]: r.get("rundung") for r in cfg["regeln"]}
    rows = []
    for p in sorted(glob.glob("pipeline/runs/produktion/*/report.json")):
        rid = os.path.basename(os.path.dirname(p))
        r = json.load(open(p))
        ca = r.get("catala_a")
        if not ca:
            print(f"{rid:40} KEIN catala_a"); continue
        res = lint(rid, ca, rundung_by.get(rid))
        rows.append(res)
        mark = "FLAG" if res["flag"] else ("cut-only" if res["cut"] else "-")
        extra = f" cut={res['cut']}" if res["cut"] else ""
        b = f" B={len(res['money_x_decimal'])}" if res["money_x_decimal"] else ""
        print(f"{rid:40} {mark:9}{extra}{b}")
    flagged = [r["rid"] for r in rows if r["flag"]]
    print("\n=== FLAGGED:", flagged, "===")
    print("=== cut-only (A ohne B, kein Flag):",
          [r["rid"] for r in rows if r["cut"] and not r["flag"]], "===")
    # Detail der geflaggten
    for r in rows:
        if r["flag"]:
            print(f"\n--- {r['rid']} B-Treffer ---")
            for i, ln, frag in r["money_x_decimal"]:
                print(f"  Zeile {i}: {frag!r}  in  {ln}")


if __name__ == "__main__":
    main()
