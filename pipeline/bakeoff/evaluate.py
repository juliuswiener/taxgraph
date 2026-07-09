"""G2 bake-off evaluation: aggregate run reports into reports/gate-g2.md.

Consumes the report.json files a bake-off run writes (one per candidate per
pairing) and computes the metrics from Nachtrag §5. Decision rule: lowest
escalation rate wins; cost is only a tiebreaker.

Run (after the bake-off, which Julius releases separately):
    pipeline/.venv/bin/python pipeline/bakeoff/evaluate.py pipeline/runs
"""

from __future__ import annotations

import glob
import json
import os
import sys
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))


def load_reports(runs_dir: str) -> list[dict]:
    out = []
    for p in glob.glob(os.path.join(runs_dir, "**", "report.json"), recursive=True):
        try:
            out.append(json.load(open(p, encoding="utf-8")))
        except json.JSONDecodeError:
            print(f"skip unreadable report: {p}", file=sys.stderr)
    return out


def _gate(r: dict, name: str) -> str | None:
    for g in r.get("gates", []):
        if g["name"] == name:
            return g["status"]
    return None


def pairing_of(r: dict) -> str:
    """Identify the pairing.

    A run aborted at a role has empty provenance, so the explicit `pairing` field
    is authoritative; the provenance slug is only a fallback for older reports.
    Otherwise every role_timeout would land in an "unknown" bucket and the
    flakiness would silently disappear from the comparison.
    """
    if r.get("pairing"):
        return r["pairing"]
    for p in r.get("provenance", []):
        if p["role"] == "formalisierer_b":
            return p["slug"].replace(" (dry-run)", "")
    return "unknown"


def rate(n: int, d: int) -> float:
    return (n / d) if d else 0.0


def _judge_missed(r: dict) -> bool:
    """Stage 2 of the round-trip metric: judge said faithful, reality disagreed.

    An assumption is *missed* when the judge reports `faithful` but a
    deterministic gate (equivalence) or the blind reference contradicts it. A
    judge that never flags anything looks perfect on stage 1 and fails here.
    """
    v = r.get("judge_verdict") or {}
    if not v or v.get("parse_error") or not v.get("faithful"):
        return False
    if _gate(r, "equivalence") == "FAIL":
        return True
    if "blind_repro_match" in r and not r["blind_repro_match"]:
        return True
    return False


def _judge_named(r: dict) -> bool:
    """Stage 1: judge explicitly named a silent assumption."""
    v = r.get("judge_verdict") or {}
    return bool(v and not v.get("parse_error") and v.get("stille_zusatzannahmen"))


def evaluate(reports: list[dict]) -> dict:
    by = defaultdict(list)
    for r in reports:
        by[pairing_of(r)].append(r)

    result = {}
    for pairing, rs in by.items():
        n = len(rs)
        syntax_ok = sum(1 for r in rs
                        if _gate(r, "syntax_a") == "PASS" and _gate(r, "syntax_b") == "PASS"
                        and _gate(r, "typecheck_a") in ("PASS", None)
                        and _gate(r, "typecheck_b") in ("PASS", None))
        equiv_div = sum(1 for r in rs if _gate(r, "equivalence") == "FAIL")
        rt_dev = sum(1 for r in rs if _gate(r, "roundtrip") == "FAIL")
        named = sum(1 for r in rs if _judge_named(r))
        missed = sum(1 for r in rs if _judge_missed(r))
        escalated = [r for r in rs if r.get("queue_status") == "flagged_for_review"]
        # Eskalation getrennt nach ausloesendem Gate
        per_gate = defaultdict(int)
        for r in escalated:
            for g in r.get("failed_gates", []):
                per_gate[g] += 1
        # Blind-Reproduktion: nur Laeufe zaehlen, die ueberhaupt vergleichbar waren.
        # Ein Build-Fehler des Kandidaten ist kein "nicht reproduziert", sondern
        # gar kein Vergleich - er wird getrennt ausgewiesen.
        blind = [r for r in rs if "blind_repro_match" in r]
        blind_ok = sum(1 for r in blind if r["blind_repro_match"])
        blind_tasks = [r for r in rs if "blind_repro_status" in r]
        blind_build_err = sum(1 for r in blind_tasks
                              if r["blind_repro_status"] in
                              ("blind_build_error", "blind_call_error", "blind_no_scope",
                               "blind_runtime_error"))
        blind_sig_mismatch = sum(1 for r in blind_tasks
                                 if r["blind_repro_status"] == "blind_signature_mismatch")
        blind_ref_err = sum(1 for r in blind_tasks
                            if r["blind_repro_status"] == "blind_ref_error")
        approved = [r for r in rs if r.get("queue_status", "").startswith("verified")]
        cost = sum(r.get("total_cost_usd", 0.0) for r in rs)
        # Clerk-Gate n/a-Anteil (kein EStH/BMF-Rechenbeispiel)
        clerk_na = sum(1 for r in rs if _gate(r, "clerk") == "SKIP")

        # Provider-Flakiness getrennt von Modellqualitaet: Timeouts/Retries je
        # Rolle und Provider, plus Laeufe, die an einer Rolle abgebrochen sind.
        role_timeouts = [r for r in rs if r.get("queue_status") in
                         ("role_timeout", "role_error")]
        run_errors = [r for r in rs if r.get("queue_status") == "run_error"]
        transport = {"retries": 0, "timeouts": 0, "rate_limits": 0, "errors": 0}
        by_provider: dict = defaultdict(lambda: dict.fromkeys(transport, 0))
        for r in rs:
            t = r.get("transport") or {}
            for k in transport:
                transport[k] += t.get(k, 0)
            for prov, counts in (t.get("by_provider") or {}).items():
                for k, v in counts.items():
                    by_provider[prov][k] += v
        by_role_timeout = defaultdict(int)
        for r in role_timeouts:
            by_role_timeout[f"{r.get('failed_role')} ({r.get('failed_slug')})"] += 1

        result[pairing] = {
            "n": n,
            "syntaxvaliditaet": rate(syntax_ok, n),
            "aequivalenz_divergenzrate": rate(equiv_div, n),
            "roundtrip_abweichungsrate": rate(rt_dev, n),
            # zweistufig: benannt (gut) vs verpasst (schlecht)
            "annahme_benannt": rate(named, n),
            "annahme_verpasst": rate(missed, n),
            "blind_reproduktion": rate(blind_ok, len(blind)) if blind else None,
            "blind_build_error_rate": rate(blind_build_err, len(blind_tasks)) if blind_tasks else None,
            "blind_ref_error_rate": rate(blind_ref_err, len(blind_tasks)) if blind_tasks else None,
            "blind_signature_mismatch_rate": rate(blind_sig_mismatch, len(blind_tasks)) if blind_tasks else None,
            "run_error_rate": rate(len(run_errors), n),
            "eskalationsrate": rate(len(escalated), n),
            "eskalation_je_gate": {k: rate(v, n) for k, v in sorted(per_gate.items())},
            "clerk_gate_na_anteil": rate(clerk_na, n),
            "role_timeout_rate": rate(len(role_timeouts), n),
            "transport": transport,
            "transport_by_provider": {k: dict(v) for k, v in by_provider.items()},
            "role_timeouts_by_role": dict(by_role_timeout),
            "kosten_gesamt_usd": round(cost, 6),
            "kosten_pro_approved_usd": round(cost / len(approved), 6) if approved else None,
        }
    return result


def to_markdown(res: dict) -> str:
    L = ["# Gate G2: Bake-off-Auswertung\n",
         "Entscheid: niedrigste Eskalationsrate gewinnt; Kosten nur als Tiebreaker.\n"]
    if not res:
        L.append("Keine Laeufe gefunden. Der Bake-off wurde noch nicht gestartet "
                 "(Freigabe durch Julius steht aus).\n")
        return "\n".join(L)

    cols = ["n", "syntaxvaliditaet", "aequivalenz_divergenzrate",
            "roundtrip_abweichungsrate", "annahme_benannt", "annahme_verpasst",
            "blind_reproduktion", "blind_build_error_rate", "eskalationsrate",
            "kosten_pro_approved_usd"]
    L.append("| Paarung (Formalisierer B) | " + " | ".join(cols) + " |")
    L.append("|" + "---|" * (len(cols) + 1))
    for pairing, m in sorted(res.items()):
        cells = []
        for c in cols:
            v = m[c]
            cells.append("n/a" if v is None else (f"{v:.3f}" if isinstance(v, float) else str(v)))
        L.append(f"| {pairing} | " + " | ".join(cells) + " |")

    L.append("\n`annahme_benannt` = der Judge hat eine stille Zusatzannahme selbst "
             "benannt (gut). `annahme_verpasst` = der Judge hielt die Formalisierung "
             "fuer treu, aber Aequivalenz oder die Blind-Referenz widersprechen "
             "(schlecht). Ein Judge, der nie etwas meldet, sieht in Spalte 1 gut aus "
             "und faellt in Spalte 2 durch.\n")

    L.append("\n`blind_reproduktion` zaehlt nur Laeufe, die ueberhaupt vergleichbar "
             "waren. Ein Kandidat, der nicht baut, ist kein 'nicht reproduziert', "
             "sondern gar kein Vergleich - er steht in `blind_build_error_rate`.\n")

    L.append("\n## Eskalation getrennt nach ausloesendem Gate\n")
    gates = sorted({g for m in res.values() for g in m["eskalation_je_gate"]})
    if gates:
        L.append("| Paarung | " + " | ".join(gates) + " |")
        L.append("|" + "---|" * (len(gates) + 1))
        for pairing, m in sorted(res.items()):
            row = [f"{m['eskalation_je_gate'].get(g, 0.0):.3f}" for g in gates]
            L.append(f"| {pairing} | " + " | ".join(row) + " |")
    else:
        L.append("Keine Eskalationen.\n")

    L.append("\nClerk-Gate n/a-Anteil (kein EStH/BMF-Rechenbeispiel; Aequivalenz + "
             "Round-Trip tragen):\n")
    for pairing, m in sorted(res.items()):
        L.append(f"- `{pairing}`: {m['clerk_gate_na_anteil']:.3f}")

    L.append("\n## Provider-Flakiness (Transport, nicht Modellqualitaet)\n")
    L.append("Timeouts, Retries und Rate-Limits sagen etwas ueber den Hoster, nicht "
             "ueber das Modell. Sie fliessen deshalb nicht in die Entscheidung ein.\n")
    L.append("| Paarung | role_timeout_rate | retries | timeouts | rate_limits | errors |")
    L.append("|---|---|---|---|---|---|")
    for pairing, m in sorted(res.items()):
        t = m["transport"]
        L.append(f"| {pairing} | {m['role_timeout_rate']:.3f} | {t['retries']} | "
                 f"{t['timeouts']} | {t['rate_limits']} | {t['errors']} |")
    for pairing, m in sorted(res.items()):
        if m["transport_by_provider"]:
            L.append(f"\n`{pairing}` je Provider:")
            for prov, c in sorted(m["transport_by_provider"].items()):
                L.append(f"- {prov}: retries={c['retries']} timeouts={c['timeouts']} "
                         f"rate_limits={c['rate_limits']} errors={c['errors']}")
        if m["role_timeouts_by_role"]:
            L.append(f"\n`{pairing}` abgebrochene Rollen: "
                     + ", ".join(f"{k} x{v}" for k, v in m["role_timeouts_by_role"].items()))

    ranked = sorted(res.items(), key=lambda kv: (kv[1]["eskalationsrate"],
                                                 kv[1]["kosten_pro_approved_usd"] or 0.0))
    rates = [m["eskalationsrate"] for m in res.values()]
    spread = max(rates) - min(rates)
    saturated = min(rates) >= 0.85   # praktisch jeder Lauf eskaliert
    too_close = spread < 0.10        # Unterschied im Rauschen

    L.append("\n## Empfehlung\n")
    if saturated or too_close:
        L.append("**Kein Entscheid moeglich.** Die Entscheidungsmetrik traegt nicht:\n")
        if saturated:
            L.append(f"- Die Eskalationsrate ist gesaettigt (Minimum {min(rates):.3f}). "
                     f"Praktisch jeder Lauf wird eskaliert, also trennt sie die "
                     f"Paarungen nicht.")
        if too_close:
            L.append(f"- Der Abstand betraegt nur {spread:.3f} und liegt bei "
                     f"n={ranked[0][1]['n']} Tasks je Paarung im Rauschen.")
        L.append("\nEin Sieger waere hier ein Artefakt der Metrik, kein Befund ueber "
                 "die Modelle. Vor einer Entscheidung muss die Saettigungsursache "
                 "behoben und der Bake-off wiederholt werden.\n")
        L.append("Zur Orientierung, ohne Entscheidungscharakter:\n")
        for name, m in ranked:
            L.append(f"- `{name}`: eskalation {m['eskalationsrate']:.3f}, "
                     f"syntax {m['syntaxvaliditaet']:.3f}, "
                     f"aequivalenz-divergenz {m['aequivalenz_divergenzrate']:.3f}, "
                     f"kosten ${m['kosten_gesamt_usd']:.4f}")
    else:
        L.append(f"**`{ranked[0][0]}`** (Eskalationsrate "
                 f"{ranked[0][1]['eskalationsrate']:.3f}; Kosten nur Tiebreaker). "
                 f"Entscheidung trifft Julius.\n")
    return "\n".join(L)


def main() -> int:
    runs_dir = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "pipeline", "runs")
    reports = load_reports(runs_dir)
    res = evaluate(reports)
    md = to_markdown(res)
    out = os.path.join(ROOT, "reports", "gate-g2.md")
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with open(out, "w", encoding="utf-8") as f:
        f.write(md)
    print(md)
    print(f"wrote {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
