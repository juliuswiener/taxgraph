"""Phase-3 Produktions-Runner: Regeln durch die eingefrorene Gate-Kaskade.

Keine Paarungen mehr - die Besetzung steht seit Gate G2 fest (pipeline/models.yaml).
Je Regel entsteht ein Report unter pipeline/runs/produktion/<rule_id>/ mit den
Catala-Quellen beider Formalisierer, dem Judge-Verdikt und allen Gate-Ergebnissen.

    python pipeline/produktion/run.py --dry-run   # Verdrahtung, kein Key, keine Kosten
    python pipeline/produktion/run.py             # echt (braucht OPENROUTER_API_KEY)
    python pipeline/produktion/run.py --regate    # nur die deterministischen Gates
                                                  # neu rechnen, aus gespeicherten
                                                  # Quellen, ohne Modellkosten

`--regate` ist der Grund, warum die Kaskade `catala_a` mitschreibt: wenn spaeter
Test-Seeds nachgereicht werden, muss das Clerk-Gate neu laufen - aber kein Modell
noch einmal bezahlt werden. Der Judge-Teil des Reports bleibt dabei unangetastet.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
import traceback

import yaml

HERE = os.path.dirname(os.path.abspath(__file__))
PIPELINE = os.path.dirname(HERE)
ROOT = os.path.dirname(PIPELINE)
sys.path.insert(0, PIPELINE)
sys.path.insert(0, ROOT)

from yamlstrict import load_yaml, DuplicateKeyError  # noqa: E402
from cascade import run_candidate  # noqa: E402
from client import mask_key, RoleCallError  # noqa: E402
from quellen import build_norm_text, QuellenFehler  # noqa: E402
import gates as G  # noqa: E402
import judge as J  # noqa: E402
import grenzfaelle as GF  # noqa: E402

OUT_ROOT = os.path.join(PIPELINE, "runs", "produktion")


def build_candidate(rule: dict) -> dict:
    sig = rule["signature"]
    # Multi-Source: Gesetz + auslegende Quellen, getrennt etikettiert. Zitatanker
    # und Auszuege werden hier gegen die eingefrorenen Dateien geprueft - ein
    # Verstoss bricht ab, bevor ein Modell laeuft.
    norm_text, quellen_meta = build_norm_text(rule, ROOT)
    return {
        "id": rule["rule_id"],
        "norm_text": norm_text,
        "quellen": quellen_meta,
        "exclude_rule_ids": rule.get("exclude_rule_ids", []),
        "signature": sig,
        "output_field": sig["output"],
        "input_types": sig["inputs"],
        "raster": rule["raster"],
        "test_seed": rule.get("test_seed", "none"),
        # Phase 3: das Clerk-Test-Gate ist Pflicht. Fehlen Seeds, faellt es.
        "test_gate_required": True,
        "geltungsbedingungen": rule.get("geltungsbedingungen", []),
        "output_type": "money",
    }


def judge_gates(verdict: dict, cand: dict) -> dict:
    """Die drei Judge-Gates aus einem gespeicherten Verdikt.

    Ein unlesbares oder abgeschnittenes Verdikt darf sie nicht UEBERSPRINGEN:
    sonst bleiben alte PASS-Werte im Report stehen und die Regel sieht gruen aus,
    obwohl ihr Urteil nie zustande kam. Genau so stand § 9 Abs. 4a zwischenzeitlich
    auf `verified_bedingt`, waehrend der Judge in Wahrheit am Token-Limit
    abgeschnitten worden war.
    """
    if not verdict:
        return {}
    if verdict.get("parse_error") or verdict.get("truncated"):
        det = ("judge answer truncated at max_tokens" if verdict.get("truncated")
               else "judge output not valid JSON")
        return {n: G.GateResult(n, G.FAIL, det)
                for n in ("roundtrip", "scope_gap", "geltungsbereich", "grenzfall")}
    return {"scope_gap": G.scope_gap_gate(verdict),
            "geltungsbereich": G.geltungsbereich_gate(verdict, cand),
            "grenzfall": G.grenzfall_gate(verdict),
            "roundtrip": G.roundtrip_gate(verdict, cand)}


def regate(rules: list[dict]) -> int:
    """Deterministische Gates aus gespeicherten Quellen neu rechnen. Keine Modelle."""
    changed = 0
    for rule in rules:
        path = os.path.join(OUT_ROOT, rule["rule_id"], "report.json")
        if not os.path.exists(path):
            print(f"  {rule['rule_id']}: kein Report - erst einen echten Lauf fahren")
            continue
        rep = json.load(open(path, encoding="utf-8"))
        src, mod = rep.get("catala_a"), rep.get("module_name")
        if not src:
            print(f"  {rule['rule_id']}: kein catala_a gespeichert, uebersprungen")
            continue
        cand = build_candidate(rule)
        fresh = {"syntax_a": G.syntax_gate(src, mod),
                 "typecheck_a": G.typecheck_gate(src, mod),
                 "clerk": G.clerk_gate(src, mod, cand, ROOT)}
        if rep.get("catala_b"):
            fresh["equivalence"] = G.equivalence_gate(src, rep["catala_b"], cand)
        # scope_gap und geltungsbereich sind ebenfalls deterministisch: sie lesen
        # das gespeicherte Judge-Verdikt, kein Modell laeuft erneut.
        fresh.update(judge_gates(rep.get("judge_verdict") or {}, cand))
        vorhanden = {g["name"] for g in rep["gates"]}
        for g in rep["gates"]:
            if g["name"] in fresh:
                new = fresh[g["name"]]
                if (g["status"], g["detail"]) != (new.status, new.detail):
                    changed += 1
                g["status"], g["detail"] = new.status, new.detail
        # Ein neu eingefuehrtes Gate steht in aelteren Reports noch nicht drin und
        # wuerde sonst nie erscheinen.
        for name, new in fresh.items():
            if name not in vorhanden:
                rep["gates"].append({"name": name, "status": new.status,
                                     "detail": new.detail})
                changed += 1
        rep["failed_gates"] = [g["name"] for g in rep["gates"]
                               if g["status"] == G.FAIL and not g["name"].endswith("_first")]
        rep["queue_status"] = _queue_status(rep["gates"], rule,
                                            rep.get("judge_verdict"))
        rep["bedingungen"] = [b["bedingung"] for b in rule.get("geltungsbedingungen", [])]
        # normalisiert: aeltere Verdikte tragen nackte Strings ohne Mapping
        rep["annahmen_mapping"] = [G._normalize_annahme(a) for a in
                                   (rep.get("judge_verdict") or {}).get("stille_zusatzannahmen", [])]
        rep["regated"] = True
        json.dump(rep, open(path, "w", encoding="utf-8"), ensure_ascii=False, indent=2)
        bed = f" unter {len(rep['bedingungen'])} Bedingung(en)" if rep["bedingungen"] else ""
        print(f"  {rule['rule_id']}: {rep['queue_status']}{bed} "
              f"(offene Gates: {', '.join(rep['failed_gates']) or 'keine'})")
    print(f"\n{changed} Gate-Ergebnis(se) geaendert, keine Modellkosten.")
    return 0


def redo_a(rules: list[dict], dry_run: bool) -> int:
    """Formalisierer A neu laufen lassen, B aus dem Report wiederverwenden.

    Sonnets Numeric-Tower-Fehler ist ein Muster (G2 und p33_3, ueberlebt die
    Reparaturrunde). Statt ihn weiter zu messen, bekommen beide Formalisierer ein
    Cheatsheet aus den geprueften Idiomen der Handregeln - symmetrisch, damit der
    Vergleich A/B fair bleibt.

    Nur A wird neu formalisiert. Der Judge laeuft mit, weil sein Verdikt A's
    Quelltext beurteilt: ein altes Verdikt zu einem neuen Quelltext waere eine
    Luege im Report. B bleibt unangetastet - das spart den teureren Teil und haelt
    die Aequivalenzpruefung gegen genau dieselbe zweite Formalisierung.
    """
    import roles as R
    from client import OpenRouterClient
    from provenance import load_roles

    roles, models_hash = load_roles()
    for rule in rules:
        path = os.path.join(OUT_ROOT, rule["rule_id"], "report.json")
        if not os.path.exists(path):
            print(f"  {rule['rule_id']}: kein Report - erst einen vollen Lauf fahren")
            continue
        rep = json.load(open(path, encoding="utf-8"))
        src_b = rep.get("catala_b")
        if not src_b:
            print(f"  {rule['rule_id']}: kein catala_b gespeichert, uebersprungen")
            continue

        cand = build_candidate(rule)
        client = OpenRouterClient(dry_run=dry_run)
        exclude = set(cand["exclude_rule_ids"])
        cost = 0.0
        print(f"\n=== redo-a :: {rule['rule_id']} ===", flush=True)

        claims, p = R.extract(client, roles["worker"], cand["norm_text"], models_hash, exclude)
        cost += p.cost_usd
        a_text, pa = R.formalize(client, roles["formalisierer_a"], cand["norm_text"],
                                 claims, models_hash, exclude, signature=cand["signature"])
        cost += pa.cost_usd
        mod_a, src_a = G.extract_catala(a_text)
        syn, tc = G.syntax_gate(src_a, mod_a), G.typecheck_gate(src_a, mod_a)
        first = (syn.status, tc.status)
        repaired = False
        if src_a and G.FAIL in first:
            msg = syn.detail if syn.status == G.FAIL else tc.detail
            r_text, pr = R.repair(client, roles["formalisierer_a"], src_a, msg,
                                  models_hash, exclude, cand["signature"])
            cost += pr.cost_usd
            r_mod, r_src = G.extract_catala(r_text)
            if r_src:
                repaired = True
                mod_a, src_a = r_mod, r_src
                syn, tc = G.syntax_gate(src_a, mod_a), G.typecheck_gate(src_a, mod_a)

        verdict, jprov, jkosten = J.judge_regel(
            client, roles["judge"], cand["norm_text"], src_a or "", cand["signature"],
            cand["geltungsbedingungen"], models_hash,
            dauersplitter=GF.dauersplitter(rule["rule_id"]))
        cost += jkosten

        fresh = {"syntax_a_first": G.GateResult("", first[0], "erster Versuch"),
                 "typecheck_a_first": G.GateResult("", first[1], "erster Versuch"),
                 "syntax_a": syn, "typecheck_a": tc,
                 "equivalence": G.equivalence_gate(src_a, src_b, cand),
                 **judge_gates(verdict, cand),
                 "clerk": G.clerk_gate(src_a, mod_a, cand, ROOT)}
        vorhanden = {g["name"] for g in rep["gates"]}
        for g in rep["gates"]:
            if g["name"] in fresh:
                g["status"], g["detail"] = fresh[g["name"]].status, fresh[g["name"]].detail
        for name, new in fresh.items():
            if name not in vorhanden:
                rep["gates"].append({"name": name, "status": new.status, "detail": new.detail})
        rep["catala_a"], rep["module_name"] = src_a, mod_a
        rep["judge_verdict"] = verdict
        rep["backlog"] = G.unabhaengige_gaps(verdict)
        rep["repaired"] = {"a": repaired, "b": rep.get("repaired", {}).get("b", False)}
        rep["judge_lauf_id"] = verdict.get("lauf_id")
        rep["judge_timestamp"] = verdict.get("timestamp")
        rep["judge_instability"] = verdict.get("judge_instability", {})
        rep["failed_gates"] = [g["name"] for g in rep["gates"]
                               if g["status"] == G.FAIL and not g["name"].endswith("_first")]
        rep["queue_status"] = _queue_status(rep["gates"], rule, verdict)
        rep["provenance"] += [x.to_dict() for x in (p, pa)] + jprov
        rep["total_cost_usd"] = round(rep.get("total_cost_usd", 0.0) + cost, 6)
        json.dump(rep, open(path, "w", encoding="utf-8"), ensure_ascii=False, indent=2)
        gates = " ".join(f"{k}={v.status}" for k, v in fresh.items()
                         if not k.endswith("_first"))
        print(f"  {rep['queue_status']} | +${cost:.4f} | repaired_a={repaired}\n  {gates}",
              flush=True)
    return 0


def redo_judge(rules: list[dict], dry_run: bool) -> int:
    """Nur den Judge neu laufen lassen. Formalisierungen bleiben unangetastet.

    Seit dem Protokolldekret 2026-07-10 ist der Judge dekomponiert: Inventar mit
    Mehrheits-Mitgliedschaft, dann ein Mini-Call je Pruef-Item mit drei Stimmen.
    Ein 2:1-Split auf einem blockierenden Gate eskaliert.
    """
    from client import OpenRouterClient
    from provenance import load_roles

    roles, models_hash = load_roles()
    for rule in rules:
        path = os.path.join(OUT_ROOT, rule["rule_id"], "report.json")
        if not os.path.exists(path):
            print(f"  {rule['rule_id']}: kein Report"); continue
        rep = json.load(open(path, encoding="utf-8"))
        src_a = rep.get("catala_a")
        if not src_a:
            print(f"  {rule['rule_id']}: kein catala_a"); continue
        cand = build_candidate(rule)
        client = OpenRouterClient(dry_run=dry_run)
        verdict, jprov, kosten = J.judge_regel(
            client, roles["judge"], cand["norm_text"], src_a, cand["signature"],
            cand["geltungsbedingungen"], models_hash,
            dauersplitter=GF.dauersplitter(rule["rule_id"]))

        fresh = judge_gates(verdict, cand)
        vorhanden = {g["name"] for g in rep["gates"]}
        for g in rep["gates"]:
            if g["name"] in fresh:
                g["status"], g["detail"] = fresh[g["name"]].status, fresh[g["name"]].detail
        for name, new in fresh.items():
            if name not in vorhanden:
                rep["gates"].append({"name": name, "status": new.status, "detail": new.detail})
        rep["judge_verdict"] = verdict
        rep["judge_lauf_id"] = verdict.get("lauf_id")
        rep["judge_timestamp"] = verdict.get("timestamp")
        rep["judge_instability"] = verdict.get("judge_instability", {})
        rep["annahmen_mapping"] = verdict.get("stille_zusatzannahmen", [])
        rep["backlog"] = G.unabhaengige_gaps(verdict) if not verdict.get("parse_error") else []
        rep["bedingungen"] = [b["bedingung"] for b in rule.get("geltungsbedingungen", [])]
        rep["failed_gates"] = [g["name"] for g in rep["gates"]
                               if g["status"] == G.FAIL and not g["name"].endswith("_first")]
        rep["queue_status"] = _queue_status(rep["gates"], rule, verdict)
        rep["provenance"] += jprov
        rep["total_cost_usd"] = round(rep.get("total_cost_usd", 0.0) + kosten, 6)
        json.dump(rep, open(path, "w", encoding="utf-8"), ensure_ascii=False, indent=2)

        inst = verdict.get("judge_instability") or {}
        splits = len(inst.get("item_splits", []))
        ohne = len(inst.get("items_ohne_mehrheit", []))
        mapped = sum(1 for a in rep["annahmen_mapping"] if a.get("bedingung_id"))
        print(f"  {rule['rule_id']}: {rep['queue_status']} | +${kosten:.4f} | "
              f"Annahmen {mapped}/{len(rep['annahmen_mapping'])} | splits={splits} "
              f"ohne_mehrheit={ohne} | offen: {', '.join(rep['failed_gates']) or 'keine'}",
              flush=True)
    return 0


def _queue_status(gates: list[dict], rule: dict | None = None,
                  verdict: dict | None = None) -> str:
    st = [g["status"] for g in gates if not g["name"].endswith("_first")]
    if G.FAIL in st:
        return "flagged_for_review"
    # Ein `freigabe: blockiert` im Manifest ueberstimmt gruene Gates. Sonst koennte
    # eine Regel, deren Redundanz-Gate nie getragen hat, still auf `verified`
    # rutschen, sobald das letzte technische Gate gruen wird.
    if (rule or {}).get("freigabe") == "blockiert":
        return "freigabe_blockiert"
    if G.SKIP in st:
        return "verified_partial (toolchain pending)"
    # Ein 2:1-Split auf einem blockierenden Gate ist kein stilles PASS: der Split
    # ist Information und eskaliert.
    if verdict and J.hat_split_auf_blockierendem_gate(verdict):
        return "judge_split_eskaliert"
    # Statusehrlichkeit: eine Regel mit Geltungsbedingungen ist nie schlicht
    # "verified". Sie gilt nur unter ihren Bedingungen; Phase 5 muss sie als
    # Fragen stellen, und die Engine darf die Regel bei Verletzung nicht feuern.
    if (rule or {}).get("geltungsbedingungen"):
        return "verified_bedingt"
    return "verified"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--only", help="eine rule_id")
    ap.add_argument("--force", action="store_true", help="fertige Reports neu rechnen")
    ap.add_argument("--regate", action="store_true",
                    help="nur deterministische Gates neu rechnen, ohne Modellkosten")
    ap.add_argument("--redo-judge", action="store_true",
                    help="nur den Judge neu laufen lassen (Template-Wechsel)")
    ap.add_argument("--redo-a", action="store_true",
                    help="nur Formalisierer A (und den Judge) neu laufen lassen, "
                         "B aus dem Report wiederverwenden")
    args = ap.parse_args()

    try:
        cfg = load_yaml(os.path.join(HERE, "rules.yaml"))
    except DuplicateKeyError as e:
        raise SystemExit(str(e))
    rules = cfg["regeln"]
    if args.only:
        rules = [r for r in rules if r["rule_id"] == args.only]
    else:
        # Regeln, deren Zuschnitt offen ist, laufen nicht: ein Approval-Versuch
        # waere sinnlos, solange die Signatur den Norm-Ausschnitt verfehlt.
        offen = [r["rule_id"] for r in rules if r.get("status") == "zuschnitt_offen"]
        if offen:
            print(f"uebersprungen (zuschnitt_offen): {', '.join(offen)}")
        rules = [r for r in rules if r.get("status") != "zuschnitt_offen"]
    if not rules:
        raise SystemExit(f"keine Regel {args.only!r} im Manifest")

    # Quellen-Vorbedingung: Zitatanker und Auszuege gegen die eingefrorenen
    # Dateien pruefen, bevor ein Modell laeuft. Ein Verstoss ist ein Abbruch.
    try:
        for r in rules:
            build_candidate(r)
    except QuellenFehler as e:
        raise SystemExit(f"Quellen-Gate: {e}")
    print(f"Quellen-Gate ok: {len(rules)} Regel(n), alle Zitatanker und Auszuege "
          f"woertlich in den eingefrorenen Quellen")

    if args.regate:
        return regate(rules)
    if args.redo_judge:
        return redo_judge(rules, args.dry_run)
    if args.redo_a:
        return redo_a(rules, args.dry_run)

    total_cost, t0 = 0.0, time.time()
    for rule in rules:
        rid = rule["rule_id"]
        d = os.path.join(OUT_ROOT, rid)
        done = os.path.join(d, "report.json")
        if os.path.exists(done) and not args.force:
            print(f"=== {rid} :: skip (checkpoint) ===")
            total_cost += json.load(open(done, encoding="utf-8")).get("total_cost_usd", 0.0)
            continue

        print(f"\n=== {rid} ===", flush=True)
        os.makedirs(d, exist_ok=True)
        try:
            res = run_candidate(build_candidate(rule), dry_run=args.dry_run)
        except RoleCallError as e:
            json.dump({"candidate_id": rid, "queue_status": e.kind,
                       "failed_role": e.role, "failed_slug": e.slug,
                       "providers": e.providers, "reason": e.reason,
                       "gates": [], "failed_gates": [], "judge_verdict": {},
                       "backlog": [], "provenance": [], "total_cost_usd": 0.0},
                      open(done, "w", encoding="utf-8"), ensure_ascii=False, indent=2)
            print(f"  {e.kind}: {e.role} ({e.slug}) -> weiter", flush=True)
            continue
        except Exception as e:  # noqa: BLE001
            json.dump({"candidate_id": rid, "queue_status": "run_error",
                       "reason": mask_key(f"{type(e).__name__}: {e}")[:300],
                       "traceback": mask_key(traceback.format_exc())[-800:],
                       "gates": [], "failed_gates": [], "judge_verdict": {},
                       "backlog": [], "provenance": [], "total_cost_usd": 0.0},
                      open(done, "w", encoding="utf-8"), ensure_ascii=False, indent=2)
            print("  run_error:", mask_key(str(e))[:150], flush=True)
            continue

        queue = res.queue_status
        if rule.get("freigabe") == "blockiert" and queue.startswith("verified"):
            queue = "freigabe_blockiert"
        report = {
            "candidate_id": rid,
            "norm": rule.get("norm"),
            "freigabe": rule.get("freigabe", "offen"),
            # Herkunft der Modell-Eingabe: welche eingefrorenen Quellen, welcher Typ
            "quellen": build_candidate(rule)["quellen"],
            "geltungsbedingungen": rule.get("geltungsbedingungen", []),
            "dry_run": res.dry_run,
            "models_yaml_hash": res.models_yaml_hash,
            "queue_status": queue,
            "bedingungen": res.bedingungen,
            "annahmen_mapping": (res.judge_verdict or {}).get("stille_zusatzannahmen", []),
            "gates": res.gates_dict(),
            "failed_gates": [g.name for g in res.gate_results
                             if g.status == G.FAIL and not g.name.endswith("_first")],
            "repaired": res.repaired,
            "judge_verdict": res.judge_verdict,
            # Punkt 5 des Protokolldekrets: ein Verdikt-Report nennt den Judge-Lauf,
            # der ihn erzeugt hat. Ein Gate ohne frisches Verdikt hat keinen Zustand.
            "judge_lauf_id": (res.judge_verdict or {}).get("lauf_id"),
            "judge_timestamp": (res.judge_verdict or {}).get("timestamp"),
            "judge_instability": (res.judge_verdict or {}).get("judge_instability", {}),
            "backlog": res.backlog,
            "transport": res.transport,
            "provenance": res.provenance,
            "total_cost_usd": round(res.total_cost_usd, 6),
            "catala_a": res.catala_a,
            "catala_b": res.catala_b,
            "module_name": res.module_name,
        }
        json.dump(report, open(done, "w", encoding="utf-8"), ensure_ascii=False, indent=2)
        total_cost += res.total_cost_usd
        gates = " ".join(f"{g.name}={g.status}" for g in res.gate_results
                         if not g.name.endswith("_first"))
        print(f"  {res.queue_status} | ${res.total_cost_usd:.4f} | "
              f"backlog +{len(res.backlog)}\n  {gates}", flush=True)

    print(f"\nTOTAL ${total_cost:.4f}, wall {time.time()-t0:.0f}s")
    print(f"Reports unter {OUT_ROOT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
