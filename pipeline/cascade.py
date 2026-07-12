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
import grenzfaelle as GF
import item_registry as IR


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
    discoveries: list = field(default_factory=list)  # neue Items -> Triage
    bedingungen: list = field(default_factory=list)  # IDs, gelten am Status

    def gates_dict(self):
        return [{"name": g.name, "status": g.status, "detail": g.detail}
                for g in self.gate_results]


def run_candidate(candidate: dict, dry_run: bool | None = None,
                  fixtures_dir: str | None = None,
                  role_overrides: dict | None = None,
                  skip_judge: bool = False) -> CascadeResult:
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
    zusatz = candidate.get("formalisierer_zusatz", "")   # B1-Experiment, sonst leer
    mod_a, src_a = _formalize_repair(client, roles["formalisierer_a"], "a", norm,
                                     claims_text, models_hash, exclude, sig, res, record, zusatz)
    mod_b, src_b = _formalize_repair(client, roles["formalisierer_b"], "b", norm,
                                     claims_text, models_hash, exclude, sig, res, record, zusatz)
    # Rundungs-Lint auf A (Dekret 2026-07-11): eine nicht deklarierte
    # Rundungsoperation geht als Compiler-artige Meldung in dieselbe Repair-Runde.
    lint = G.rundungs_lint_gate(src_a, candidate)
    if src_a and lint.status == G.FAIL:
        r_text, rp = R.repair(client, roles["formalisierer_a"], src_a, lint.detail,
                              models_hash, exclude, sig)
        record(rp)
        r_mod, r_src = G.extract_catala(r_text)
        if r_src:
            res.repaired["a"] = True
            mod_a, src_a = r_mod, r_src
            lint = G.rundungs_lint_gate(src_a, candidate)
            # syntax_a/typecheck_a stehen schon in der Liste (aus _formalize_repair)
            # und muessen die reparierte Quelle widerspiegeln.
            syn2, tc2 = G.syntax_gate(src_a, mod_a), G.typecheck_gate(src_a, mod_a)
            for g in res.gate_results:
                if g.name == "syntax_a":
                    g.status, g.detail = syn2.status, syn2.detail
                elif g.name == "typecheck_a":
                    g.status, g.detail = tc2.status, tc2.detail

    # Praezisions-Lint auf A (Klasse 5): der confident-Befund (money x decimal vor
    # finalem Cent-Schnitt) geht mit seinem decimal-Idiom als deterministisches Signal
    # in EINE Repair-Runde - Gate-Output, kein Prompt-Change. INFO/FAIL loesen die
    # Reparatur aus; der Queue-Status wird davon (Stufe 1) trotzdem nicht gekippt.
    plint = G.praezisions_lint_gate(src_a, candidate)
    if src_a and plint.status in (G.INFO, G.FAIL):
        r_text, rp = R.repair(client, roles["formalisierer_a"], src_a, plint.detail,
                              models_hash, exclude, sig)
        record(rp)
        r_mod, r_src = G.extract_catala(r_text)
        if r_src:
            res.repaired["a"] = True
            mod_a, src_a = r_mod, r_src
            plint = G.praezisions_lint_gate(src_a, candidate)
            lint = G.rundungs_lint_gate(src_a, candidate)   # reparierte Quelle spiegeln
            syn2, tc2 = G.syntax_gate(src_a, mod_a), G.typecheck_gate(src_a, mod_a)
            for g in res.gate_results:
                if g.name == "syntax_a":
                    g.status, g.detail = syn2.status, syn2.detail
                elif g.name == "typecheck_a":
                    g.status, g.detail = tc2.status, tc2.detail

    res.queue_status = "formalized"
    res.module_name, res.catala_a, res.catala_b = mod_a, src_a, src_b

    # 5. Rundungs-Lint (deterministisch, kostenlos) VOR der Aequivalenz
    res.gate_results.append(_named("rundungs_lint", lint))

    # 5a. Praezisions-Lint (Klasse 5, deterministisch, kostenlos). Stufe-1-Rollout:
    #     INFO-Befund, kippt kein Gate (Vorregistrierung 2026-07-12). Stufe 2
    #     (blockierend) erst nach Julius via G._PRAEZISION_BLOCKIEREND.
    res.gate_results.append(_named("praezisions_lint", plint))

    # 5b. extensionale Aequivalenz A vs B auf dem Input-Raster
    res.gate_results.append(G.equivalence_gate(src_a, src_b, candidate))

    # 6. Round-Trip-Diff (Judge auf A). Der Judge sieht die Signatur und bewertet
    #    nur, was innerhalb ihrer Grenze liegt; Norm-Teile ausserhalb -> scope_gap.
    # skip_judge: nur fuer Experimente, die den Judge nicht brauchen (B1: nur
    #    catala_a/b + clerk). Produktion laesst den Judge immer laufen (default False).
    if src_a and not skip_judge:
        # Dekomponierter Judge: Inventar (3 Laeufe, Mehrheits-Mitgliedschaft) und
        # ein Mini-Call je Pruef-Item mit 3 Stimmen. Ein Parse-Fehler ist keine
        # Stimme; ohne Mehrheit gilt das Item konservativ.
        verdict, jprov, jkosten = J.judge_regel(
            client, roles["judge"], norm, src_a, sig,
            candidate.get("geltungsbedingungen"), models_hash,
            dauersplitter=GF.dauersplitter(candidate["id"]))
        res.provenance.extend(jprov)
        res.total_cost_usd += jkosten
        res.judge_verdict = verdict
        # Registry-Ratsche (Stufe 4): der Judge ist Detektor. Die Gates pruefen
        # gegen die Registry, nicht gegen diesen Lauf; neue Funde landen in der
        # Discovery-Queue und kippen kein Gate.
        rule_min = {"geltungsbedingungen": candidate.get("geltungsbedingungen") or []}
        judge_gates, discoveries = IR.gates_fuer(candidate["id"], rule_min, verdict)
        for name in ("roundtrip", "scope_gap", "geltungsbereich", "grenzfall", "defekt", "discovery"):
            if name in judge_gates:
                res.gate_results.append(_named(name, judge_gates[name]))
        res.discoveries = discoveries
        if not verdict.get("parse_error"):
            res.backlog = G.unabhaengige_gaps(verdict)
    else:
        # skip_judge MIT gueltiger A-Quelle: die Judge-Gates sind SKIPPED (nicht FAIL) -
        # das Judge-Verdikt fehlt bewusst (deterministischer Struktur/clerk-Lauf).
        # Ohne A-Quelle bleibt es FAIL "no A source".
        judge_skipped = bool(skip_judge and src_a)
        st = G.SKIP if judge_skipped else G.FAIL
        det = "judge uebersprungen (skip_judge)" if judge_skipped else "no A source"
        for name in ("roundtrip", "scope_gap", "geltungsbereich", "grenzfall", "defekt"):
            res.gate_results.append(G.GateResult(name, st, det))
        if judge_skipped:
            res.judge_verdict = {"skipped": True}

    # 7. Clerk-Tests
    res.gate_results.append(G.clerk_gate(src_a, mod_a, candidate))

    res.transport = client.transport_summary()

    # verify all stamps share one models.yaml hash (run validity)
    hashes = {p["models_yaml_hash"] for p in res.provenance}
    if len(hashes) > 1:
        res.queue_status = "invalid_mixed_models_yaml"
        return res

    res.queue_status = _queue_status(res.gate_results, candidate, res.discoveries,
                                     judge_skipped=bool(skip_judge and src_a))
    res.bedingungen = [b["bedingung"] for b in (candidate.get("geltungsbedingungen") or [])]
    return res


def _queue_status(gate_results, candidate, discoveries, judge_skipped=False) -> str:
    """Queue-Entscheidung der Registry-Ratsche.

    Das `discovery`-Gate ist SKIP bei neuen Funden - das darf NICHT als
    "toolchain pending" gelten. Neue Funde routen in die Triage-Queue, kippen aber
    kein deterministisches Gate (Punkt 3). Judge-Splits sind informativ und
    entscheiden nichts mehr.

    Falschgrün-Sperre (Instructor-Leitplanke 2026-07-12): ein skip_judge-Lauf hat KEIN
    Judge-Verdikt und darf daher NIE Richtung verified/verified_bedingt laufen - egal wie
    gruen die deterministischen Gates sind. Er bleibt strukturell/vorlaeufig, bis der
    Judge-Nachzug (redo-judge) die Ratsche schliesst.
    """
    echte = [g for g in gate_results
             if not g.name.endswith("_first") and g.name != "discovery"]
    if G.FAIL in [g.status for g in echte]:
        return "flagged_for_review"
    if discoveries:
        return "discovery_triage"
    if judge_skipped:
        return "strukturgeprueft_judge_offen"
    if G.SKIP in [g.status for g in echte]:
        return "verified_partial (toolchain pending)"
    if candidate.get("geltungsbedingungen"):
        return "verified_bedingt"
    return "verified"


def _named(name: str, g: G.GateResult) -> G.GateResult:
    g.name = name
    return g


def _formalize_repair(client, role, tag: str, norm: str, claims_text: str,
                      models_hash: str, exclude, sig, res, record, zusatz: str = ""):
    """Formalise, then repair once iff syntax or typecheck failed.

    Exactly one repair round, only on a compiler failure, input = the candidate's
    own source plus the verbatim compiler diagnostic. Both formalisers get the
    same treatment. The first-pass gates are kept as `syntax_<tag>_first` /
    `typecheck_<tag>_first` so the report can separate first-pass from post-repair
    validity; the unsuffixed gates are the ones the cascade acts on.
    """
    text, p = R.formalize(client, role, norm, claims_text, models_hash,
                          exclude, signature=sig, zusatz=zusatz)
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
