"""Gate cascade orchestration.

Order per Nachtrag 2026-07-09 §4:
  Extraktion (worker)
  -> Doppelformalisierung (A und B, identisches Template + Few-Shots)
  -> Syntax-Gate (tree-sitter/heuristic)
  -> Compiler-Typecheck
  -> extensionale Aequivalenz (A vs B auf Input-Raster)
  -> Round-Trip-Diff (Judge)
  -> Clerk-Tests
  -> Review-Queue-Status

Every model output is provenance-stamped; a run is valid only if all stamps
carry the same models.yaml hash. Compiler gates SKIP cleanly when the toolchain
is absent (wiring stays testable in dry-run).
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass, field

from client import OpenRouterClient
from provenance import load_roles, Provenance, now_iso
import roles as R
import gates as G
import judge as J


@dataclass
class CascadeResult:
    candidate_id: str
    dry_run: bool
    models_yaml_hash: str
    gate_results: list = field(default_factory=list)
    provenance: list = field(default_factory=list)
    module_name: str | None = None
    catala_a: str | None = None
    catala_b: str | None = None
    total_cost_usd: float = 0.0
    queue_status: str = "extracted"
    judge_verdict: dict = field(default_factory=dict)
    transport: dict = field(default_factory=dict)
    repaired: dict = field(default_factory=dict)   # {"a": bool, "b": bool}
    backlog: list = field(default_factory=list)    # unabhaengige scope_gaps
    bedingungen: list = field(default_factory=list)  # IDs, gelten am Status

    def gates_dict(self):
        return [{"name": g.name, "status": g.status, "detail": g.detail}
                for g in self.gate_results]


def run_candidate(candidate: dict, dry_run: bool | None = None,
                  fixtures_dir: str | None = None,
                  role_overrides: dict | None = None) -> CascadeResult:
    """Run one rule candidate through the cascade.

    candidate: {id, norm_text, exclude_rule_ids?, signature?, ...}
    role_overrides: {role_name: {"slug": ..., "providers": [...]}} for a bake-off
    pairing. The models.yaml hash still stamps every output, so a run stays
    traceable to one frozen config.
    """
    roles, models_hash = load_roles()
    for rname, ov in (role_overrides or {}).items():
        if rname in roles:
            roles[rname].slug = ov["slug"]
            roles[rname].providers = list(ov["providers"])
    client = OpenRouterClient(dry_run=dry_run, fixtures_dir=fixtures_dir)
    exclude = set(candidate.get("exclude_rule_ids", []))
    norm = candidate["norm_text"]
    res = CascadeResult(candidate_id=candidate["id"], dry_run=client.dry_run,
                        models_yaml_hash=models_hash)

    def record(prov: Provenance):
        res.provenance.append(prov.to_dict())
        res.total_cost_usd += prov.cost_usd

    # 1. Extraktion (worker)
    claims_text, p = R.extract(client, roles["worker"], norm, models_hash, exclude)
    record(p)

    # 2.-4. Doppelformalisierung (A und B), je mit genau einer Reparaturrunde
    #       auf Syntax-/Typecheck-Fehler. Symmetrisch fuer beide Formalisierer.
    sig = candidate.get("signature")
    mod_a, src_a = _formalize_repair(client, roles["formalisierer_a"], "a", norm,
                                     claims_text, models_hash, exclude, sig, res, record)
    mod_b, src_b = _formalize_repair(client, roles["formalisierer_b"], "b", norm,
                                     claims_text, models_hash, exclude, sig, res, record)
    res.queue_status = "formalized"
    res.module_name, res.catala_a, res.catala_b = mod_a, src_a, src_b

    # 5. extensionale Aequivalenz A vs B auf dem Input-Raster
    res.gate_results.append(G.equivalence_gate(src_a, src_b, candidate))

    # 6. Round-Trip-Diff (Judge auf A). Der Judge sieht die Signatur und bewertet
    #    nur, was innerhalb ihrer Grenze liegt; Norm-Teile ausserhalb -> scope_gap.
    if src_a:
        # Dekomponierter Judge: Inventar (3 Laeufe, Mehrheits-Mitgliedschaft) und
        # ein Mini-Call je Pruef-Item mit 3 Stimmen. Ein Parse-Fehler ist keine
        # Stimme; ohne Mehrheit gilt das Item konservativ.
        verdict, jprov, jkosten = J.judge_regel(
            client, roles["judge"], norm, src_a, sig,
            candidate.get("geltungsbedingungen"), models_hash)
        res.provenance.extend(jprov)
        res.total_cost_usd += jkosten
        res.judge_verdict = verdict
        if verdict.get("parse_error"):
            det = "judge inventory produced no valid verdict"
            for name in ("roundtrip", "scope_gap", "geltungsbereich"):
                res.gate_results.append(G.GateResult(name, G.FAIL, det))
        else:
            res.gate_results.append(G.roundtrip_gate(verdict, candidate))
            # scope_gap ist informativ (zaehlt, eskaliert nicht). Blockierend ist
            # geltungsbereich: es faellt auf jeden wirkt_hinein OHNE abdeckende
            # Bedingung. Eine deklarierte Annahme ist keine stille Annahme.
            res.gate_results.append(G.scope_gap_gate(verdict))
            res.gate_results.append(G.geltungsbereich_gate(verdict, candidate))
            res.backlog = G.unabhaengige_gaps(verdict)
    else:
        res.gate_results.append(G.GateResult("roundtrip", G.FAIL, "no A source"))
        res.gate_results.append(G.GateResult("scope_gap", G.FAIL, "no A source"))
        res.gate_results.append(G.GateResult("geltungsbereich", G.FAIL, "no A source"))

    # 7. Clerk-Tests
    res.gate_results.append(G.clerk_gate(src_a, mod_a, candidate))

    res.transport = client.transport_summary()

    # verify all stamps share one models.yaml hash (run validity)
    hashes = {p["models_yaml_hash"] for p in res.provenance}
    if len(hashes) > 1:
        res.queue_status = "invalid_mixed_models_yaml"
        return res

    # queue decision - the `_first` gates are diagnostics, not gates: a run that
    # only failed before its repair round must not be flagged for that.
    statuses = [g.status for g in res.gate_results if not g.name.endswith("_first")]
    bedingungen = [b["bedingung"] for b in (candidate.get("geltungsbedingungen") or [])]
    if G.FAIL in statuses:
        res.queue_status = "flagged_for_review"
    elif G.SKIP in statuses:
        res.queue_status = "verified_partial (toolchain pending)"
    elif J.hat_split_auf_blockierendem_gate(res.judge_verdict):
        # Ein 2:1-Split auf einem blockierenden Gate ist kein stilles PASS.
        # Der Split ist Information und gehoert vor Julius.
        res.queue_status = "judge_split_eskaliert"
    elif bedingungen:
        # Statusehrlichkeit: eine Teilformalisierung ist nie schlicht "verified".
        # Sie gilt nur unter ihren Bedingungen, und die stehen am Status.
        res.queue_status = "verified_bedingt"
    else:
        res.queue_status = "verified"
    res.bedingungen = bedingungen
    return res


def _named(name: str, g: G.GateResult) -> G.GateResult:
    g.name = name
    return g


def _formalize_repair(client, role, tag: str, norm: str, claims_text: str,
                      models_hash: str, exclude, sig, res, record):
    """Formalise, then repair once iff syntax or typecheck failed.

    Exactly one repair round, only on a compiler failure, input = the candidate's
    own source plus the verbatim compiler diagnostic. Both formalisers get the
    same treatment. The first-pass gates are kept as `syntax_<tag>_first` /
    `typecheck_<tag>_first` so the report can separate first-pass from post-repair
    validity; the unsuffixed gates are the ones the cascade acts on.
    """
    text, p = R.formalize(client, role, norm, claims_text, models_hash,
                          exclude, signature=sig)
    record(p)
    mod, src = G.extract_catala(text)
    syn, tc = G.syntax_gate(src, mod), G.typecheck_gate(src, mod)
    res.gate_results.append(_named(f"syntax_{tag}_first", _copy(syn)))
    res.gate_results.append(_named(f"typecheck_{tag}_first", _copy(tc)))
    res.repaired[tag] = False

    if src and G.FAIL in (syn.status, tc.status):
        msg = syn.detail if syn.status == G.FAIL else tc.detail
        r_text, rp = R.repair(client, role, src, msg, models_hash, exclude, sig)
        record(rp)
        r_mod, r_src = G.extract_catala(r_text)
        if r_src:
            res.repaired[tag] = True
            mod, src = r_mod, r_src
            syn, tc = G.syntax_gate(src, mod), G.typecheck_gate(src, mod)

    res.gate_results.append(_named(f"syntax_{tag}", syn))
    res.gate_results.append(_named(f"typecheck_{tag}", tc))
    return mod, src


def _copy(g: G.GateResult) -> G.GateResult:
    return G.GateResult(g.name, g.status, g.detail)


def write_report(res: CascadeResult, out_dir: str | None = None) -> str:
    out_dir = out_dir or os.path.join(os.path.dirname(__file__), "runs",
                                      f"{res.candidate_id}_{now_iso().replace(':', '')}")
    os.makedirs(out_dir, exist_ok=True)
    failed = [g.name for g in res.gate_results
              if g.status == G.FAIL and not g.name.endswith("_first")]
    report = {
        "candidate_id": res.candidate_id,
        "dry_run": res.dry_run,
        "models_yaml_hash": res.models_yaml_hash,
        "queue_status": res.queue_status,
        "module_name": res.module_name,
        "gates": res.gates_dict(),
        # Eskalation getrennt nach Gate auswertbar machen
        "failed_gates": failed,
        # zweistufige Round-Trip-Wertung (Stufe 2 in evaluate.py)
        "judge_verdict": res.judge_verdict,
        "repaired": res.repaired,
        "backlog": res.backlog,
        "provenance": res.provenance,
        "total_cost_usd": round(res.total_cost_usd, 6),
    }
    path = os.path.join(out_dir, "report.json")
    with open(path, "w", encoding="utf-8") as f:
        json.dump(report, f, ensure_ascii=False, indent=2)
    return path
