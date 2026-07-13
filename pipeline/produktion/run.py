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
import hashlib
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
import item_registry as IR  # noqa: E402

OUT_ROOT = os.path.join(PIPELINE, "runs", "produktion")


def _archive_report(path: str) -> str | None:
    """Vor dem Ueberschreiben (--force / redo) die alte report.json daneben sichern.
    Ein role_timeout oder ein schlechteres Formalisierungs-Artefakt darf ein
    besseres nicht spurlos ersetzen (2026-07-11: ein 5/6-A ging so verloren).
    Suffix = mtime der alten Datei (deterministisch aus der Datei selbst)."""
    if not os.path.exists(path):
        return None
    arch = f"{path}.{int(os.path.getmtime(path))}"
    try:
        os.replace(path, arch)
        return arch
    except OSError:
        return None


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
        "rundung": rule.get("rundung", []),
        "output_type": "money",
        # hinweis-Kanal (Julius-Freigabe 2026-07-12, Messgrundlage
        # reports/review/2026-07-12-b1-rolespy-ergebnis.md): optionaler kuratierter
        # Bearbeitungshinweis (Spec/Catala-Idiom, NIE paraphrasierter Gesetzestext),
        # geht als eigener Block VOR die Formalisier-Anweisung. Default leer ->
        # Prompt byte-identisch, kein Regress fuer Regeln ohne hinweis.
        "formalisierer_zusatz": rule.get("hinweis", ""),
    }


def hinweis_provenance(rule: dict) -> dict:
    """Auditierbarer Nachweis des im Lauf verwendeten hinweis (Instructor 2026-07-12).

    Der hinweis geht ueber `formalisierer_zusatz` in den Formalisierer-Prompt. Wird er
    spaeter in rules.yaml geaendert, muss ein alter report.json noch zeigen, WAS damals
    gesendet wurde. Leerer hinweis -> leere Provenance (kein Kanal aktiv), damit "kein
    hinweis" im Report sofort sichtbar ist statt als Hash von "".
    """
    h = rule.get("hinweis", "") or ""
    return {"hinweis": h,
            "hinweis_sha256": hashlib.sha256(h.encode("utf-8")).hexdigest() if h else ""}


def judge_gates(verdict: dict, cand: dict) -> tuple[dict, list]:
    """Registry-getriebene Judge-Gates + Discovery-Liste aus einem Verdikt.

    Die Gates pruefen gegen die Registry, nicht gegen das frische Verdikt (Stufe 4).
    Ein unlesbares oder abgeschnittenes Verdikt darf sie nicht UEBERSPRINGEN: sonst
    bleiben alte PASS-Werte im Report stehen und die Regel sieht gruen aus, obwohl
    ihr Urteil nie zustande kam. Rueckgabe: ({gate_name: GateResult}, discoveries).
    """
    if not verdict:
        return {}, []
    rule_min = {"geltungsbedingungen": cand.get("geltungsbedingungen") or []}
    return IR.gates_fuer(cand["id"], rule_min, verdict)


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
                 "rundungs_lint": G.rundungs_lint_gate(src, cand),
                 "clerk": G.clerk_gate(src, mod, cand, ROOT)}
        if rep.get("catala_b"):
            fresh["equivalence"] = G.equivalence_gate(src, rep["catala_b"], cand)
        # scope_gap und geltungsbereich sind ebenfalls deterministisch: sie lesen
        # das gespeicherte Judge-Verdikt, kein Modell laeuft erneut.
        jg, disc = judge_gates(rep.get("judge_verdict") or {}, cand)
        fresh.update(jg)
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
        rep["discoveries"] = disc
        rep["queue_status"] = _queue_status(rep["gates"], rule,
                                            rep.get("judge_verdict"), disc)
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

        # Rundungs-Lint: Befund geht als Compiler-artige Meldung in DIESELBE
        # Repair-Maschinerie (Dekret 2026-07-11, Punkt 2), kein neuer Prompt-Versuch.
        lint = G.rundungs_lint_gate(src_a, cand)
        if src_a and lint.status == G.FAIL:
            r_text, pr = R.repair(client, roles["formalisierer_a"], src_a, lint.detail,
                                  models_hash, exclude, cand["signature"])
            cost += pr.cost_usd
            r_mod, r_src = G.extract_catala(r_text)
            if r_src:
                repaired = True
                mod_a, src_a = r_mod, r_src
                syn, tc = G.syntax_gate(src_a, mod_a), G.typecheck_gate(src_a, mod_a)
                lint = G.rundungs_lint_gate(src_a, cand)

        verdict, jprov, jkosten = J.judge_regel(
            client, roles["judge"], cand["norm_text"], src_a or "", cand["signature"],
            cand["geltungsbedingungen"], models_hash,
            dauersplitter=GF.dauersplitter(rule["rule_id"]))
        cost += jkosten

        jg, disc = judge_gates(verdict, cand)
        fresh = {"syntax_a_first": G.GateResult("", first[0], "erster Versuch"),
                 "typecheck_a_first": G.GateResult("", first[1], "erster Versuch"),
                 "syntax_a": syn, "typecheck_a": tc,
                 "rundungs_lint": lint,
                 "equivalence": G.equivalence_gate(src_a, src_b, cand),
                 **jg,
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
        rep["discoveries"] = disc
        rep["queue_status"] = _queue_status(rep["gates"], rule, verdict, disc)
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

        fresh, disc = judge_gates(verdict, cand)
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
        rep["discoveries"] = disc
        rep["queue_status"] = _queue_status(rep["gates"], rule, verdict, disc)
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
                  verdict: dict | None = None, discoveries: list | None = None) -> str:
    # Das `discovery`-Gate ist SKIP bei neuen Funden - kein "toolchain pending".
    st = [g["status"] for g in gates
          if not g["name"].endswith("_first") and g["name"] != "discovery"]
    # Falschgruen-Sperre (budget_abbruch/Abbruch-Reports): keine bewertbaren Gates =
    # die Regel lief nie. Ohne Gates NIE verified, egal welcher Pfad hier landet.
    if not st:
        return "unbewertet"
    if G.FAIL in st:
        return "flagged_for_review"
    # Ein `freigabe: blockiert` im Manifest ueberstimmt gruene Gates.
    if (rule or {}).get("freigabe") == "blockiert":
        return "freigabe_blockiert"
    # Neue Funde routen in die Triage-Queue (Registry-Ratsche, Punkt 3). Sie kippen
    # kein deterministisches Gate, aber die Regel ist nicht fertig, bis Julius sie
    # triagiert hat.
    if discoveries:
        return "discovery_triage"
    # Falschgruen-Sperre (spiegelt cascade._queue_status): ein skip_judge-Report hat kein
    # Judge-Verdikt und darf NIE Richtung verified*/verified_partial laufen, egal wie gruen die
    # deterministischen Gates sind. Muss VOR dem SKIP->verified_partial-Zweig stehen, sonst
    # laeuft er faelschlich auf "verified_partial". strukturgeprueft bis zum Judge-Nachzug.
    if (verdict or {}).get("skipped"):
        return "strukturgeprueft_judge_offen"
    if G.SKIP in st:
        return "verified_partial (toolchain pending)"
    # Statusehrlichkeit: eine Regel mit Geltungsbedingungen ist nie schlicht
    # "verified". verified_bedingt = alle REGISTRIERTEN Items abgedeckt +
    # deterministische Gates gruen.
    if (rule or {}).get("geltungsbedingungen"):
        return "verified_bedingt"
    return "verified"


def _estimate_cost(rule: dict) -> float:
    """Kalibrierte Vorab-Kostenschaetzung eines Kaskaden-Laufs (deterministisch, kein LLM).
    Empirie Charge 8-11: multi-quellige Regeln (>= 2 Quellen, mehr Judge-Tokens) ~$0.15,
    1-quellige ~$0.07. Konservativ (eher zu hoch), damit der Cap nicht knapp gerissen wird."""
    return 0.15 if len(rule.get("quellen", [])) >= 2 else 0.07


def _ist_checkpoint(prev: dict) -> bool:
    """Zaehlt ein vorhandener Report als Checkpoint (Skip beim Wiederanlauf)? Ja - AUSSER es
    ist ein budget_abbruch: der markiert eine NIE gelaufene Regel (Pre-Call-Cap-Stopp) und muss
    beim Wiederanlauf (erhoehter Cap) neu anlaufen. Sonst wuerde die nie verifizierte Regel als
    'done' still uebersprungen (Checkpoint-Falle, F1)."""
    return prev.get("queue_status") != "budget_abbruch"


def _budget_abbruch_report(rid: str, total_cost: float, est: float, cap: float) -> dict:
    """Report eines nicht gestarteten Laufs (Pre-Call-Cap ueberschritten). queue_status
    budget_abbruch, gates leer - die Falschgruen-Sperre in _queue_status haelt das auf
    'unbewertet', falls der Report je durch die Queue-Logik laeuft."""
    return {"candidate_id": rid, "queue_status": "budget_abbruch",
            "reason": f"Pre-Call-Cap ${cap:.2f} wuerde gerissen: kumuliert ${total_cost:.4f} "
                      f"+ Schaetzung ${est:.2f} > Cap. Regel {rid} nicht gestartet.",
            "gates": [], "failed_gates": [], "judge_verdict": {},
            "backlog": [], "provenance": [], "total_cost_usd": 0.0}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--only", help="eine rule_id")
    ap.add_argument("--force", action="store_true", help="fertige Reports neu rechnen")
    ap.add_argument("--skip-judge", action="store_true",
                    help="Judge-Rolle ueberspringen (deterministischer Struktur/clerk-Lauf, "
                         "z.B. bei Judge-Provider-Ausfall). queue_status bleibt strukturell "
                         "'strukturgeprueft_judge_offen' - NIE verified (Falschgruen-Sperre). "
                         "Judge-Nachzug spaeter via --redo-judge.")
    ap.add_argument("--regate", action="store_true",
                    help="nur deterministische Gates neu rechnen, ohne Modellkosten")
    ap.add_argument("--redo-judge", action="store_true",
                    help="nur den Judge neu laufen lassen (Template-Wechsel)")
    ap.add_argument("--redo-a", action="store_true",
                    help="nur Formalisierer A (und den Judge) neu laufen lassen, "
                         "B aus dem Report wiederverwenden")
    ap.add_argument("--cost-cap", type=float, default=None,
                    help="Vorab-Kosten-Cap in USD fuer den gesamten Lauf. Deterministischer "
                         "Pre-Call-Check: uebersteigt kumulierte Ist-Kosten + kalibrierte "
                         "Schaetzung des naechsten Calls den Cap, wird die Regel NICHT "
                         "gestartet (queue_status budget_abbruch) und der Lauf abgebrochen - "
                         "nie stiller Weiterlauf. Default: kein Cap (Instructor-Freigabe je Charge). "
                         "Grenze (N1): RoleCallError/run_error-Pfade akkumulieren die Partial-Kosten "
                         "des gescheiterten Calls nicht (total_cost_usd 0.0), der Cap undercountet auf "
                         "Fehlerpfaden daher marginal.")
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
        # zuschnitt_ersetzt: durch Teilregeln abgeloeste Monolithen (Historie bleibt).
        # handgeschrieben: als Handregel unter rules/estg/ migriert (aus den Pipeline-
        # Gates raus wie p32a/p04/p09; Registry + Geltungsbedingungen gelten weiter).
        SKIP_STATUS = ("zuschnitt_offen", "zuschnitt_ersetzt", "handgeschrieben")
        uebersprungen = [r["rule_id"] for r in rules if r.get("status") in SKIP_STATUS]
        if uebersprungen:
            print(f"uebersprungen (status): {', '.join(uebersprungen)}")
        rules = [r for r in rules if r.get("status") not in SKIP_STATUS]
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
            prev = json.load(open(done, encoding="utf-8"))
            if _ist_checkpoint(prev):
                print(f"=== {rid} :: skip (checkpoint) ===")
                total_cost += prev.get("total_cost_usd", 0.0)
                continue
            # budget_abbruch-Report: nie gelaufene Regel, neu anlaufen lassen (F1).
            print(f"=== {rid} :: budget_abbruch-Report gefunden -> Neuanlauf ===")

        print(f"\n=== {rid} ===", flush=True)
        os.makedirs(d, exist_ok=True)
        arch = _archive_report(done)   # --force: alte report.json sichern, nicht ueberschreiben
        if arch:
            print(f"  (alte report.json -> {os.path.basename(arch)})", flush=True)
        # Pre-Call-Kosten-Cap (deterministisch): wuerde der naechste Call den Cap reissen,
        # Regel NICHT starten -> budget_abbruch + Lauf beenden (nie stiller Weiterlauf).
        if args.cost_cap is not None:
            est = _estimate_cost(rule)
            if total_cost + est > args.cost_cap:
                json.dump(_budget_abbruch_report(rid, total_cost, est, args.cost_cap),
                          open(done, "w", encoding="utf-8"), ensure_ascii=False, indent=2)
                print(f"  budget_abbruch: kumuliert ${total_cost:.4f} + est ${est:.2f} "
                      f"> cap ${args.cost_cap:.2f} - {rid} nicht gestartet, Rest uebersprungen",
                      flush=True)
                break
        try:
            res = run_candidate(build_candidate(rule), dry_run=args.dry_run,
                                skip_judge=args.skip_judge)
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
            # hinweis-Kanal-Provenance: WAS in diesem Lauf an den Formalisierer ging
            # (auditierbar, auch wenn rule["hinweis"] spaeter geaendert wird).
            **hinweis_provenance(rule),
            "geltungsbedingungen": rule.get("geltungsbedingungen", []),
            "dry_run": res.dry_run,
            "models_yaml_hash": res.models_yaml_hash,
            "queue_status": queue,
            "bedingungen": res.bedingungen,
            "discoveries": res.discoveries,
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
